#!/usr/bin/env python3
"""Helpers for run-wine-tests.sh: build vectors from the fixture's selftest
output, and check nv-call and nv-probe results against it.

Subcommands:
  call-vectors   selftest text + nm output  ->  vectors file
  call-check     selftest text + results    ->  compare bit for bit
  decode-crosscheck workdir exe...          ->  length decoder vs objdump
  extra-vectors  nm output                  ->  vectors for faults, imports,
                                                snapshots and register presets
  extra-check    results                    ->  check those
  snapshot-files nm output + folder         ->  write snapshot region files
  probe-manifest variant nm exe out [k=v..] ->  write nv-probe.txt
  probe-check    variant log out base label ->  check a probe log
  nvse-check     log out mode label         ->  check the NVSE simulation
Run with no arguments for details. Standard library only.
"""
import hashlib
import json
import os
import re
import struct
import subprocess
import sys

FAILURES = []
CHECKS = 0


def check(cond, message):
    global CHECKS
    CHECKS += 1
    if not cond:
        FAILURES.append(message)
        print(f"  FAIL: {message}")


def finish(label):
    print(f"{label}: {CHECKS - len(FAILURES)}/{CHECKS} checks passed")
    return 1 if FAILURES else 0


# ---- symbols and selftest -------------------------------------------------

def parse_nm(text):
    """Map plain function/variable names to addresses."""
    syms = {}
    for line in text.splitlines():
        m = re.match(r"^([0-9a-fA-F]{8}) ([TtDdBbRr]) (\S+)$", line.strip())
        if not m:
            continue
        name = m.group(3)
        name = name.lstrip("_@").split("@")[0]
        syms[name] = int(m.group(1), 16)
    return syms


def parse_selftest(text):
    cases = []
    for line in text.splitlines():
        if not line.startswith("CASE "):
            continue
        fields = {}
        for tok in line[5:].split(" "):
            k, _, v = tok.partition("=")
            fields[k] = v
        cases.append(fields)
    return cases


def read_jsonl(path):
    rows = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


# ---- call vectors ---------------------------------------------------------

def vector_for(case, syms):
    v = {
        "id": case["id"],
        "fn": "0x%08x" % syms[case["sym"]],
        "cc": case["cc"],
        "args": [a for a in case["args"].split(",") if a],
        "ret": case["ret"],
        "fpcw": case["fpcw"],
    }
    buffers = {}
    for k, val in case.items():
        if k.startswith("in."):
            buffers[k[3:]] = {"size": len(val) // 2, "init": val}
    v["buffers"] = buffers
    return v


def cmd_call_vectors(selftest, nm, out):
    cases = parse_selftest(open(selftest).read())
    syms = parse_nm(open(nm).read())
    with open(out, "w") as f:
        for c in cases:
            f.write(json.dumps(vector_for(c, syms)) + "\n")
    print(f"wrote {len(cases)} vectors to {out}")
    return 0


def cmd_call_check(selftest, results, label):
    cases = parse_selftest(open(selftest).read())
    rows = read_jsonl(results)
    header = rows[0]
    by_id = {r["id"]: r for r in rows if r.get("type") == "result"}
    check(header.get("type") == "header", "first line is the header")
    check(bool(header.get("cpu_vendor")) and bool(header.get("cpu_brand")), "header has the CPU vendor and brand")
    check(len(header.get("image_sha256", "")) == 64, "header has the image SHA-256")
    check(header.get("fpcw") == "0x0000027F" and header.get("mxcsr") == "0x00001F80", "header has the default control words")
    check(len(cases) > 40, f"selftest produced many cases ({len(cases)})")
    for c in cases:
        cid = c["id"]
        r = by_id.get(cid)
        if r is None:
            check(False, f"{cid}: no result")
            continue
        check(r["fault"] is None, f"{cid}: no fault ({r['fault']})")
        regs = r["regs"]
        if regs is None:
            continue
        cw = int(c["fpcw"], 16)
        check(int(regs["fcw"], 16) == cw, f"{cid}: control word after the call is {regs['fcw']}")
        # The low six bits are sticky exception flags that SSE code may set.
        check(int(regs["mxcsr"], 16) & ~0x3F == 0x1F80, f"{cid}: MXCSR control bits after the call ({regs['mxcsr']})")
        if "eax" in c:
            check(regs["eax"].lower() == "0x" + c["eax"], f"{cid}: eax {regs['eax']} vs {c['eax']}")
        if "f32" in c:
            check(r["value"]["f32_bits"].lower() == "0x" + c["f32"], f"{cid}: f32 bits {r['value']['f32_bits']} vs {c['f32']}")
        if "f64" in c:
            check(r["value"]["f64_bits"].lower() == "0x" + c["f64"], f"{cid}: f64 bits {r['value']['f64_bits']} vs {c['f64']}")
        if "st0" in c:
            st0 = regs["st0"]
            check(st0 is not None and st0["raw"] == c["st0"], f"{cid}: raw ST0 {st0 and st0['raw']} vs {c['st0']}")
        if "xmm0" in c:
            check(regs["xmm0"] == c["xmm0"], f"{cid}: xmm0 {regs['xmm0']} vs {c['xmm0']}")
        for k, val in c.items():
            if k.startswith("out."):
                check(r["buffers"].get(k[4:]) == val, f"{cid}: buffer {k[4:]} {r['buffers'].get(k[4:])} vs {val}")
        x87 = c["ret"] in ("f32_x87", "f64_x87")
        check(regs["fpu_depth"] == (1 if x87 else 0), f"{cid}: fpu depth {regs['fpu_depth']}")
        stack_args = r["stack_bytes"]
        want = "none" if stack_args == 0 else ("caller" if c["cc"] == "cdecl" else "callee")
        check(regs["cleanup"] == want, f"{cid}: stack cleanup {regs['cleanup']} (cc {c['cc']}, {stack_args} bytes)")
    return finish(label)


# ---- extra vectors --------------------------------------------------------

def extra_vectors(syms):
    def v(i, sym, cc="cdecl", args=(), ret="i32", **kw):
        d = {"id": i, "fn": "0x%08x" % syms[sym], "cc": cc, "args": list(args), "ret": ret}
        d.update(kw)
        return d
    return [
        v("fault_null_read", "fx_deref", args=["ptr:null"]),
        v("fault_ud2", "fx_ud2"),
        v("fault_div0", "fx_divide", args=["i32:5", "i32:0"]),
        v("fault_write", "fx_fill", args=["ptr:0x00000010", "i32:1"], ret="void"),
        v("import_trap", "fx_calls_import", args=["u32:5"]),
        v("after_faults", "fx_add3", args=["i32:1", "i32:2", "i32:3"]),
        v("global_5", "fx_use_global", args=["i32:5"]),
        v("read_ptr", "fx_read_ptr"),
        v("regs_ecx_5", "fx_tiny_raw", regs={"ecx": "0x5"}),
        v("regs_ecx_0", "fx_tiny_raw", regs={"ecx": "0x0"}),
        v("mix_declared_cdecl", "fx_mix", args=["i32:3", "u32:16"]),
        v("add3_declared_stdcall", "fx_add3", cc="stdcall", args=["i32:1", "i32:2", "i32:3"]),
        v("preset_conflict", "fx_this_sum", cc="thiscall", args=["ptr:0x1000", "i32:1"], regs={"ecx": "0x5"}),
        v("buffer_offset", "fx_this_sum", cc="thiscall", args=["ptr:obj+4", "i32:2"],
          buffers={"obj": {"size": 16, "init": "ffffffff" + "05000000" + "03000000" + "00000000"}}),
    ]


def cmd_extra_vectors(nm, out):
    syms = parse_nm(open(nm).read())
    with open(out, "w") as f:
        for d in extra_vectors(syms):
            f.write(json.dumps(d) + "\n")
        # Lines that must be reported as errors without stopping the run.
        f.write("this is not json\n")
        f.write(json.dumps({"id": "bad_buffer", "fn": "0x00401000", "args": ["ptr:nope"]}) + "\n")
        f.write(json.dumps({"id": "bad_cc", "fn": "0x00401000", "cc": "pascal"}) + "\n")
        f.write(json.dumps({"id": "bad_this", "fn": "0x00401000", "cc": "thiscall", "args": ["f32:1.0"]}) + "\n")
        f.write(json.dumps({"id": "bad_mxcsr", "fn": "0x%08x" % syms["fx_add3"], "mxcsr": "0x10000"}) + "\n")
        f.write(json.dumps({"id": "last_good", "fn": "0x%08x" % syms["fx_add3"], "args": ["i32:2", "i32:2", "i32:2"]}) + "\n")
    return 0


def cmd_snapshot_files(nm, folder):
    syms = parse_nm(open(nm).read())
    import os
    os.makedirs(folder, exist_ok=True)
    # fx_global_k = 1000, and fx_ptr_global -> a heap-like region at 0x0A000000.
    with open(os.path.join(folder, "%08x.bin" % syms["fx_global_k"]), "wb") as f:
        f.write(struct.pack("<i", 1000))
    with open(os.path.join(folder, "%08x.bin" % syms["fx_ptr_global"]), "wb") as f:
        f.write(struct.pack("<I", 0x0A000100))
    with open(os.path.join(folder, "0a000100.bin"), "wb") as f:
        f.write(struct.pack("<i", 51966))
    return 0


def cmd_extra_check(results, snapshot, label):
    rows = read_jsonl(results)
    by_id = {r["id"]: r for r in rows if r.get("type") == "result"}
    errors = [r for r in rows if r.get("type") == "error"]

    def fault_of(i):
        return by_id[i]["fault"]

    f = fault_of("fault_null_read")
    check(f and f["name"] == "access_violation" and f["access"] == "read" and f["address"] == "0x00000000", f"null read fault: {f}")
    check(by_id["fault_null_read"]["regs"] is None, "no registers after a fault")
    f = fault_of("fault_ud2")
    check(f and f["name"] == "illegal_instruction", f"ud2 fault: {f}")
    f = fault_of("fault_div0")
    check(f and f["name"] == "integer_divide_by_zero", f"divide fault: {f}")
    f = fault_of("fault_write")
    check(f and f["access"] == "write" and 0x10 <= int(f["address"], 16) < 0x20, f"write fault: {f}")
    f = fault_of("import_trap")
    check(f and f["kind"] == "import" and f["name"] == "KERNEL32.dll!GetCurrentProcessId", f"import trap: {f}")
    check(f and int(f["caller"], 16) != 0, "import trap reports the caller")
    r = by_id["after_faults"]
    check(r["fault"] is None and r["regs"]["eax"] == "0x00000002", "the batch survives faults")
    check(r["regs"]["fcw"] == "0x0000027F" and r["regs"]["fpu_depth"] == 0, "FPU is clean after recovery")

    r = by_id["global_5"]
    want = 1005 if snapshot else 12
    check(r["fault"] is None and r["value"]["i32"] == want, f"global value {r['value']} (snapshot={snapshot})")
    r = by_id["read_ptr"]
    want = 51966 if snapshot else -1
    check(r["fault"] is None and r["value"]["i32"] == want, f"pointer read {r['value']} (snapshot={snapshot})")

    check(by_id["regs_ecx_5"]["regs"]["eax"] == "0x00000001", "preset ECX nonzero")
    check(by_id["regs_ecx_0"]["regs"]["eax"] == "0x00000000", "preset ECX zero")
    check(by_id["mix_declared_cdecl"]["regs"]["cleanup"] == "callee", "a wrongly declared cdecl is detected as callee-cleaned")
    check(by_id["add3_declared_stdcall"]["regs"]["cleanup"] == "caller", "a wrongly declared stdcall is detected as caller-cleaned")
    check(by_id["buffer_offset"]["regs"]["eax"] == "0x0000000B" or by_id["buffer_offset"]["regs"]["eax"] == "0x0000000b",
          f"buffer offset pointer: {by_id['buffer_offset']['regs']['eax']}")

    msgs = " | ".join(e["error"] for e in errors)
    check(len(errors) == 6, f"six bad lines reported as errors ({len(errors)}): {msgs}")
    check("bad_buffer" in {e.get("id") for e in errors} or "unknown buffer" in msgs, "unknown buffer reported")
    check("preset_conflict" in {e.get("id") for e in errors}, "register preset conflict reported")
    check(by_id["last_good"]["regs"]["eax"] == "0x00000004" and by_id["last_good"]["fault"] is None, "the run continues after bad lines")
    return finish(label)


# ---- probe ----------------------------------------------------------------

OBJDUMP = os.environ.get("OBJDUMP", "i686-w64-mingw32-objdump")


def function_bytes(exe, addr, count=12):
    """The first `count` bytes of the function at `addr`, read with objdump."""
    out = subprocess.run(
        [OBJDUMP, "-d", "-M", "intel", "--start-address=0x%x" % addr,
         "--stop-address=0x%x" % (addr + count + 8), "--insn-width=16", exe],
        check=True, capture_output=True, text=True).stdout
    data = bytearray()
    for line in out.splitlines():
        m = re.match(r"^\s*([0-9a-f]+):\t((?:[0-9a-f]{2} )+)", line)
        if m and int(m.group(1), 16) >= addr and len(data) < count:
            data += bytes.fromhex(m.group(2).replace(" ", ""))
    return bytes(data[:count])


def hex_bytes(b):
    return " ".join("%02X" % x for x in b)


def hook_line(syms, exe, name, sym, cc="cdecl", args="", ret=None, dump=None, max_calls=None, tamper=False, regs=False):
    addr = syms[sym]
    b = bytearray(function_bytes(exe, addr))
    if tamper:
        b[4] ^= 0xFF
    parts = [f"hook {name} addr=0x{addr:08x} cc={cc} bytes=\"{hex_bytes(b)}\""]
    if args:
        parts.append(f"args={args}")
    if ret:
        parts.append(f"ret={ret}")
    if dump:
        parts.append(f"dump={dump}")
    if max_calls is not None:
        parts.append(f"max={max_calls}")
    if regs:
        parts.append("regs=1")
    return " ".join(parts)


def file_sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        h.update(f.read())
    return h.hexdigest()


def cmd_probe_manifest(variant, nm, exe, out, *opts):
    syms = parse_nm(open(nm).read())
    o = dict(x.split("=", 1) for x in opts)
    lines = ["# generated by oracle_tests.py"]
    mode = o.get("mode", "inject")
    lines.append(f"output = {o.get('output', 'probe.jsonl')}")
    lines.append(f"mode = {mode}")
    lines.append(f"flush_ms = {o.get('flush_ms', '250')}")
    if "suspend" in o:
        lines.append(f"suspend_threads = {o['suspend']}")
    if "grace" in o:
        lines.append(f"nvse_grace_ms = {o['grace']}")
    sha = file_sha256(exe)
    if variant == "badsha":
        sha = "0" * 63 + "1"
    if variant != "nosha":
        lines.append(f"host_sha256 = {sha}")
    H = lambda *a, **k: hook_line(syms, exe, *a, **k)
    if variant in ("main", "badsha", "nosha"):
        lines += [
            H("add3", "fx_add3", args="i32,i32,i32", ret="i32", regs=True),
            H("mix", "fx_mix", cc="stdcall", args="i32,u32", ret="i32", max_calls=2),
            H("this_sum", "fx_this_sum", cc="thiscall", args="ptr,i32", ret="i32", dump="ecx+0:16,arg0+4:8", regs=True),
            H("obj_set", "fx_obj_set", cc="thiscall", args="ptr,i32,i32", ret="i32", dump="ecx+0:16,esp+4:8,[esp+4]:4"),
            H("lerp", "fx_lerp", args="f32,f32,f32", ret="f32_x87"),
            H("fact", "fx_fact", args="i32", ret="i32"),
            H("reloc", "fx_reloc", args="i32", ret="i32"),
            H("thunk", "fx_thunk_sub3", args="i32,i32,i32", ret="i32"),
            H("fill_wrong_bytes", "fx_fill", args="ptr,i32", tamper=True),
        ]
    elif variant == "simple":
        lines += [H("add3", "fx_add3", args="i32,i32,i32", ret="i32")]
    elif variant == "threads":
        lines += [H("fact", "fx_fact", args="i32", ret="i32")]
    elif variant == "rel8":
        lines += [
            H("add3", "fx_add3", args="i32,i32,i32", ret="i32"),
            "hook tiny addr=0x%08x bytes=\"85 C9 74 06 B8 01 00 00 00\"" % syms["fx_tiny_raw"],
        ]
    else:
        print(f"unknown variant {variant}")
        return 2
    with open(out, "w") as f:
        f.write("\n".join(lines) + "\n")
    return 0


def load_log(path):
    rows = read_jsonl(path)
    header = rows[0]
    body = rows[1:]
    return header, [r for r in body if r["type"] == "call"], [r for r in body if r["type"] == "ret"], body


def argvals(call):
    return [a.get("v") for a in call["args"]]


def f32_bits(x):
    return "0x%08X" % struct.unpack("<I", struct.pack("<f", x))[0]


def obj_state(a, b):
    return struct.pack("<iifI", a, b, (a + b) * 0.5, 0xC0DE0000 | (a & 0xFFFF)).hex()


def check_common(header, body, n_expected_tid=1):
    check(header["type"] == "header" and header["tool"] == "nv-probe", "header line first")
    seqs = [r["seq"] for r in body]
    check(seqs == sorted(seqs) and len(set(seqs)) == len(seqs), "sequence numbers strictly increase")
    check(len({r["tid"] for r in body}) == n_expected_tid, f"{n_expected_tid} thread id(s) seen ({len({r['tid'] for r in body})})")
    check(all(isinstance(r["tick"], int) for r in body), "every record has a tick count")


def cmd_probe_check(variant, log, program_out, baseline_out, label):
    n = 5
    # Lines about waiting for an injector are not program results.
    prog = [l for l in open(program_out).read().strip().splitlines() if not l.startswith("waiting pid=")]
    base = open(baseline_out).read().strip().splitlines()
    if variant in ("main", "simple", "auto", "inject-pid", "nosuspend"):
        check(prog == base or variant == "inject-pid", f"program output unchanged by the probe: {prog} vs {base}")
    header, calls, rets, body = load_log(log)
    if variant == "main":
        check_common(header, body)
        check(header["installed"] is True and header["host_sha256_check"] == "match", "installed, host hash matched")
        hooks = {h["name"]: h for h in header["hooks"]}
        for name in ["add3", "mix", "this_sum", "obj_set", "lerp", "fact", "reloc", "thunk"]:
            check(hooks[name]["status"] == "installed", f"{name} installed")
        check(hooks["fill_wrong_bytes"]["status"] == "refused" and "expected bytes" in hooks["fill_wrong_bytes"]["reason"],
              f"wrong expected bytes refused: {hooks['fill_wrong_bytes']}")
        by = lambda name, kind: [r for r in (calls if kind == "call" else rets) if r["hook"] == name]
        # add3
        c, r = by("add3", "call"), by("add3", "ret")
        check(len(c) == n and len(r) == n, f"add3 called {len(c)} times, {len(r)} returns")
        for i, (cc_, rr) in enumerate(zip(c, r)):
            check(argvals(cc_) == [i, i + 1, 100] and [a["t"] for a in cc_["args"]] == ["i32"] * 3, f"add3[{i}] args {argvals(cc_)}")
            want = i + 2 * (i + 1) - 100
            check(rr["ret"] == want and rr["eax"] == "0x%08X" % (want & 0xFFFFFFFF) and rr["call"] == cc_["seq"], f"add3[{i}] ret {rr['ret']} want {want}")
            check("regs" in cc_ and "regs" in rr and "regs" not in by("mix", "call")[0], "regs only where asked")
        # mix: max=2
        c, r = by("mix", "call"), by("mix", "ret")
        check(len(c) == 2 and len(r) == 2, f"mix limited to 2 records ({len(c)}, {len(r)})")
        for i, (cc_, rr) in enumerate(zip(c, r)):
            check(argvals(cc_) == [i, 16], f"mix[{i}] args {argvals(cc_)}")
            want = ((i * 31) ^ 16)
            check(rr["ret"] == want, f"mix[{i}] ret {rr['ret']} want {want}")
        # this_sum
        c, r = by("this_sum", "call"), by("this_sum", "ret")
        check(len(c) == n and len(r) == n, f"this_sum called {len(c)} times")
        for i, (cc_, rr) in enumerate(zip(c, r)):
            check(cc_["args"][0]["t"] == "ptr" and cc_["args"][1]["v"] == 3, f"this_sum[{i}] args {cc_['args']}")
            check(cc_["dumps"][0].get("hex") == obj_state(i, 2 * i), f"this_sum[{i}] struct dump {cc_['dumps'][0]}")
            ptr = int(cc_["args"][0]["v"], 16)
            check(cc_["dumps"][0]["addr"] == "0x%08X" % ptr, "dump address is the this pointer")
            check(cc_["dumps"][1].get("hex") == struct.pack("<if", 2 * i, (3 * i) * 0.5).hex(), f"this_sum[{i}] arg0+4 dump {cc_['dumps'][1]}")
            check(rr["ret"] == i + 2 * i * 3, f"this_sum[{i}] ret {rr['ret']}")
            check(rr["dumps_after"][0]["hex"] == obj_state(i, 2 * i), "dumps_after repeats the state")
            # regs=1: ECX holds `this` at entry, and the stack pointer is word aligned.
            check(cc_["regs"]["ecx"] == cc_["args"][0]["v"], f"this_sum[{i}] ECX is the this pointer {cc_['regs']}")
            check(int(cc_["regs"]["esp"], 16) % 4 == 0 and set(cc_["regs"]) == {"eax", "ecx", "edx", "ebx", "esp", "ebp", "esi", "edi"}, "all eight registers logged")
            check(rr["regs"]["eax"] == rr["eax"], "registers logged at return too")
        # obj_set
        c, r = by("obj_set", "call"), by("obj_set", "ret")
        check(len(c) == n and len(r) == n, f"obj_set called {len(c)} times")
        for i, (cc_, rr) in enumerate(zip(c, r)):
            check([a["t"] for a in cc_["args"]] == ["ptr", "i32", "i32"] and argvals(cc_)[1:] == [i, 2 * i], f"obj_set[{i}] args {cc_['args']}")
            before = "00" * 16 if i == 0 else obj_state(i - 1, 2 * (i - 1))
            check(cc_["dumps"][0].get("hex") == before, f"obj_set[{i}] struct before {cc_['dumps'][0]} want {before}")
            check(cc_["dumps"][1].get("hex") == struct.pack("<ii", i, 2 * i).hex(), f"obj_set[{i}] esp+4 dump {cc_['dumps'][1]}")
            check("error" in cc_["dumps"][2] and "hex" not in cc_["dumps"][2], f"obj_set[{i}] unreadable dump reported, not crashed: {cc_['dumps'][2]}")
            check(rr["dumps_after"][0].get("hex") == obj_state(i, 2 * i), f"obj_set[{i}] struct after {rr['dumps_after'][0]}")
            check("error" in rr["dumps_after"][2], f"obj_set[{i}] unreadable dump still unreadable after")
            want = 0 if i == 0 else 3 * (i - 1)
            check(rr["ret"] == want, f"obj_set[{i}] ret {rr['ret']} want {want}")
        # lerp: x87 return
        c, r = by("lerp", "call"), by("lerp", "ret")
        check(len(c) == n and len(r) == n, f"lerp called {len(c)} times")
        for i, (cc_, rr) in enumerate(zip(c, r)):
            check([a["bits"] for a in cc_["args"]] == [f32_bits(float(i)), f32_bits(10.0), f32_bits(0.5)], f"lerp[{i}] args {cc_['args']}")
            st0 = rr["st0"]
            check(st0 is not None and st0["f64"] == (i + 10) / 2 and st0["f32"] == (i + 10) / 2, f"lerp[{i}] st0 {st0}")
        # Hooks whose stolen bytes hold a relative call and a relative jump:
        # the relocated copies must still reach the right targets.
        c, r = by("reloc", "call"), by("reloc", "ret")
        check(len(c) == n and len(r) == n, f"reloc called {len(c)} times")
        for i, (cc_, rr) in enumerate(zip(c, r)):
            check(argvals(cc_) == [i] and rr["ret"] == 2 * i + 6, f"reloc[{i}] (relocated call) ret {rr['ret']}")
        check(hooks["reloc"]["steal"] == 9, f"reloc steals 9 bytes ({hooks['reloc']['steal']})")
        c, r = by("thunk", "call"), by("thunk", "ret")
        check(len(c) == n and len(r) == n, f"thunk called {len(c)} times")
        for i, (cc_, rr) in enumerate(zip(c, r)):
            check(argvals(cc_) == [i, 1, 2] and rr["ret"] == i - 3, f"thunk[{i}] (relocated jump) ret {rr['ret']}")
        check(hooks["thunk"]["steal"] == 5, f"thunk steals 5 bytes ({hooks['thunk']['steal']})")
        # fact: nested calls come back in last-in first-out order
        facts = [x for x in body if x["hook"] == "fact"]
        expect = []
        for _ in range(n):
            expect += [("call", 4), ("call", 3), ("call", 2), ("call", 1), ("ret", 1), ("ret", 2), ("ret", 6), ("ret", 24)]
        got = [("call", argvals(x)[0]) if x["type"] == "call" else ("ret", x["ret"]) for x in facts]
        check(got == expect, f"fact nesting order: {got[:10]}...")
        seq_of_call = {}
        ok = True
        stack = []
        for x in facts:
            if x["type"] == "call":
                stack.append(x["seq"])
            else:
                ok = ok and stack and stack.pop() == x["call"]
        check(ok and not stack, "every fact return refers to its own call")
    elif variant in ("simple", "auto", "inject-pid", "nosuspend"):
        check_common(header, body)
        check(header["installed"] is True, "hooks installed")
        if variant == "nosuspend":
            check(header["suspend_threads"] is False and header["threads_suspended"] == 0, "no thread was suspended")
        want_origin = {"simple": "dllmain", "auto": "auto-thread", "inject-pid": "dllmain", "nosuspend": "dllmain"}[variant]
        check(header["origin"] == want_origin, f"origin {header['origin']}")
        want = n if variant != "inject-pid" else 3
        check(len(calls) == want and len(rets) == want, f"add3 calls {len(calls)} returns {len(rets)}, wanted {want}")
        for i, (cc_, rr) in enumerate(zip(calls, rets)):
            check(argvals(cc_) == [i, i + 1, 100] and rr["ret"] == i + 2 * (i + 1) - 100, f"call {i}")
    elif variant == "badsha":
        check(header["host_sha256_check"] == "mismatch" and header["installed"] is False, "host hash mismatch refuses everything")
        check(all(h["status"] == "refused" and "SHA-256" in h["reason"] for h in header["hooks"]) and len(header["hooks"]) == 9, "every hook refused with the reason")
        check(not calls and not rets, "no calls were logged")
        check(prog == base, f"program output unchanged: {prog} vs {base}")
    elif variant == "rel8":
        check("manifest_error" in header and "short branch" in header["manifest_error"], f"manifest error reported: {header.get('manifest_error')}")
        check(header["installed"] is False and not calls, "nothing installed after a manifest error")
        check(prog == base, "program output unchanged")
    elif variant == "threads":
        # 4 threads x 200 iterations x fact(5) recursion depth 5
        check_common(header, body, n_expected_tid=4)
        check(len(calls) == 4 * 200 * 5 and len(rets) == 4 * 200 * 5, f"threads: {len(calls)} calls, {len(rets)} returns")
        by_tid = {}
        for x in body:
            by_tid.setdefault(x["tid"], []).append(x)
        want = [("call", 5), ("call", 4), ("call", 3), ("call", 2), ("call", 1), ("ret", 1), ("ret", 2), ("ret", 6), ("ret", 24), ("ret", 120)]
        for tid, rows in by_tid.items():
            got = [("call", argvals(x)[0]) if x["type"] == "call" else ("ret", x["ret"]) for x in rows]
            check(got == want * 200, f"thread {tid}: nesting order intact over {len(rows)} records")
        check("bad=0" in " ".join(prog), f"program saw correct results: {prog}")
    return finish(label)


def cmd_nvse_check(log, program_out, mode, label):
    prog = open(program_out).read()
    if mode == "editor":
        check("query=0" in prog and "load=0" in prog, f"editor: plugin refuses to load: {prog.strip()}")
        check(not os.path.exists(log), "editor: no log written")
        return finish(label)
    check("query=1 info_version=1 name=nv-probe version=1" in prog, f"query filled PluginInfo: {prog.strip()}")
    check("load=1" in prog, "load returned true")
    rows = read_jsonl(log)
    check(sum(1 for r in rows if r["type"] == "header") == 1, "installed once")
    header = rows[0]
    check(header["origin"] == "nvse" and header["installed"] is True, f"origin {header.get('origin')}")
    calls = [r for r in rows if r["type"] == "call"]
    # nvse-sim runs the loop 3 times; with "free" the DLL is unloaded and
    # the second run (2 iterations) must not be logged.
    check(len(calls) == 3, f"three loop iterations logged ({len(calls)})")
    if mode == "free":
        check("unloaded" in prog and prog.count("loop n=") == 2, f"program ran after unload: {prog.strip()}")
    return finish(label)


def cmd_decode_crosscheck(workdir, *exes):
    """Compare the length decoder with objdump on the .text of compiled programs."""
    os.makedirs(workdir, exist_ok=True)
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    for exe in exes:
        data = open(exe, "rb").read()
        pe = struct.unpack_from("<I", data, 0x3C)[0]
        nsec = struct.unpack_from("<H", data, pe + 6)[0]
        optsz = struct.unpack_from("<H", data, pe + 20)[0]
        base = struct.unpack_from("<I", data, pe + 24 + 28)[0]
        text = None
        for i in range(nsec):
            o = pe + 24 + optsz + i * 40
            if data[o:o + 8].rstrip(b"\0") == b".text":
                vsize, va, rsize, rptr = struct.unpack_from("<IIII", data, o + 8)
                text = (va, data[rptr:rptr + min(rsize, vsize)])
        if text is None:
            print(f"{exe}: no .text section")
            return 1
        va, raw = text
        listing = subprocess.run([OBJDUMP, "-d", "-M", "intel", "--insn-width=16", exe],
                                 check=True, capture_output=True, text=True).stdout
        starts = set()
        cut = len(raw)
        for line in listing.splitlines():
            m = re.match(r"^\s*([0-9a-f]+):\t(?:[0-9a-f]{2} )+", line)
            parts = line.split("\t")
            if m and len(parts) >= 3 and parts[2].strip():
                off = int(m.group(1), 16) - (base + va)
                if not 0 <= off < len(raw):
                    continue
                if parts[2].strip().startswith(("(bad)", ".byte")):
                    # Data at the end of .text (constructor lists); stop here.
                    cut = min(cut, off)
                    continue
                starts.add(off)
        raw = raw[:cut]
        bounds = sorted(o for o in starts if o < cut) + [cut]
        code_path = os.path.join(workdir, "code.bin")
        bounds_path = os.path.join(workdir, "boundaries.txt")
        open(code_path, "wb").write(raw)
        open(bounds_path, "w").write("\n".join(map(str, bounds)) + "\n")
        run = subprocess.run(["cargo", "run", "--quiet", "--release", "-p", "nv-oracle-core", "--example", "decode_check",
                              "--", code_path, bounds_path], cwd=root, capture_output=True, text=True)
        print(f"{os.path.basename(exe)}: {run.stdout.strip()}")
        check(run.returncode == 0, f"{exe}: decoder agrees with objdump {run.stderr.strip()[:200]}")
        check(len(bounds) > 1000, f"{exe}: enough instructions to mean something ({len(bounds)})")
    return finish("decoder cross-check")


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 2
    cmd, args = argv[1], argv[2:]
    if cmd == "call-vectors":
        return cmd_call_vectors(*args)
    if cmd == "call-check":
        return cmd_call_check(*args)
    if cmd == "extra-vectors":
        return cmd_extra_vectors(*args)
    if cmd == "snapshot-files":
        return cmd_snapshot_files(*args)
    if cmd == "extra-check":
        return cmd_extra_check(args[0], args[1] == "snapshot", args[2])
    if cmd == "decode-crosscheck":
        return cmd_decode_crosscheck(*args)
    if cmd == "probe-manifest":
        return cmd_probe_manifest(*args)
    if cmd == "probe-check":
        return cmd_probe_check(*args)
    if cmd == "nvse-check":
        return cmd_nvse_check(*args)
    print(f"unknown subcommand {cmd}")
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
