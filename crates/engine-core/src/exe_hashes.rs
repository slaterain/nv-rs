// Section facts of FalloutNV.exe 1.4.0.525 (installed Steam file and the
// unpacked file agree). Virtual sizes from the section table; hashes are
// FNV-1a 64 of each section's raw bytes.
const RDATA_VSIZE: u32 = 0x001a_304c;
const RDATA_HASH: u64 = 0xabca_c329_7bf4_b2f4;
const DATA_VSIZE: u32 = 0x000e_ea9c;
const DATA_HASH: u64 = 0x543b_f6c5_3d25_c8d8;
const TLS_VSIZE: u32 = 0x0000_02c5;
const TLS_HASH: u64 = 0xd7c1_ecde_8c74_8cf7;
const CONST_VSIZE: u32 = 0x0000_001f;
const CONST_HASH: u64 = 0x3de1_acd7_29c9_f219;

// Local experimental profile, measured 2026-11-19 from the user's unmodified
// GOG offline installer 1.4.0.525(a) (55068), en-US. Executable SHA256:
// 89e7020487e80f20ddc92ea641c4433b2c6839a82f45a076e414d167d11dd0fe.
// All mapped addresses and virtual sizes match Steam. .data, .tls and CONST
// are byte-identical by the reference hashes above. In .rdata, replacing the
// single DLL name GalaxyWrp.dll with steam_api.dll restores RDATA_HASH exactly.
// Require
// this build's .text too: accepting its data does not prove gameplay parity.
const GOG_RDATA_HASH: u64 = 0x2a79_b64d_203d_51a4;
const GOG_TEXT_HASH: u64 = 0xca78_cfed_09a4_90f7;
