//! `functions_ln/ln_fn_recipe.h` (JIP LN NVSE): a recipe's skill, category, input and output
//! components and conditions, and the recipes that use a form. All 23 of its commands. The recipe
//! is read as any form, then checked with `IS_ID` (recipe 0x6A); categories with `IS_ID` 0x6B.
//!
//! The recipe's fields are checked in its loader and save (`forms::TESRecipe`); a component is
//! 8 bytes of the game's heap (quantity, item), a condition 0x1C (`forms::Condition`). Two of
//! JIP's paths would read through a null pointer; the port answers as if nothing matched there:
//! `RemoveComponent` meeting an empty node (an empty list), and `GetFormRecipeOutputs` called
//! without its optional category (JIP's `filter` is then uninitialised; the port takes none).

use super::arrays::{create_array, Element, TempElements};
use super::forms::{form_type_b6, Condition, DataHandler, ListNode, RecipeComponent, TESRecipe};
use super::{
    heap_alloc, heap_free, list_append, list_items, list_prepend, list_unlink, set_form_result,
    type_of, Handlers,
};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2254, get_recipe_input_forms),
    (0x2255, get_recipe_output_forms),
    (0x2256, get_recipe_input_count),
    (0x2257, set_recipe_input_count),
    (0x2258, get_recipe_output_count),
    (0x2259, set_recipe_output_count),
    (0x225a, get_recipe_required_skill),
    (0x225b, set_recipe_required_skill),
    (0x225c, get_recipe_required_skill_level),
    (0x225d, set_recipe_required_skill_level),
    (0x225e, replace_recipe_input_form),
    (0x225f, replace_recipe_output_form),
    (0x2260, get_form_recipes),
    (0x2261, get_form_recipe_outputs),
    (0x22d1, get_recipe_category),
    (0x22d2, set_recipe_category),
    (0x22d3, get_recipe_subcategory),
    (0x22d4, set_recipe_subcategory),
    (0x22d8, add_recipe_input_form),
    (0x22d9, add_recipe_output_form),
    (0x22da, remove_recipe_input_form),
    (0x22db, remove_recipe_output_form),
    (0x22dc, add_recipe_condition),
];

/// `DataHandler::GetSingleton` (`*0x011C3F2C`).
const DATA_HANDLER: u32 = 0x011c_3f2c;

/// Argument 0 when the read succeeded and it is a recipe (`IS_ID(recipe, TESRecipe)`).
fn recipe_arg(e: &Engine, read: &Read) -> Option<Ptr<TESRecipe>> {
    let form = read.form(0);
    (read.ok && type_of(e, form) == form_type_b6::TES_RECIPE).then(|| Ptr::new(form))
}

fn is_category(e: &Engine, form: u32) -> bool {
    type_of(e, form) == form_type_b6::TES_RECIPE_CATEGORY
}

/// The inputs (+0x3C) or the outputs (+0x44).
fn components(recipe: Ptr<TESRecipe>, outputs: bool) -> Ptr<ListNode> {
    recipe.at(if outputs {
        TESRecipe::outputs
    } else {
        TESRecipe::inputs
    })
}

fn item(e: &Engine, component: u32) -> u32 {
    e.get(
        Ptr::<RecipeComponent>::new(component),
        RecipeComponent::item,
    )
}

/// The first component holding `form` (`component && component->item == form`), 0 for none.
fn find_component(e: &Engine, list: Ptr<ListNode>, form: u32) -> u32 {
    list_items(e, list)
        .into_iter()
        .find(|&c| c != 0 && item(e, c) == form)
        .unwrap_or(0)
}

// ---- skill ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_recipe.h:
// Cmd_GetRecipeRequiredSkill_Execute and Cmd_GetRecipeRequiredSkillLevel_Execute.
/// The word as an `int`; -1 for anything but a recipe.
fn get_word(e: &mut Engine, w: &[u32], field: Field<TESRecipe, u32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let v = match recipe_arg(e, &read) {
        Some(r) => e.get(r, field) as i32,
        None => -1,
    };
    a.set_result(e, f64::from(v));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_recipe.h:
// Cmd_SetRecipeRequiredSkill_Execute and Cmd_SetRecipeRequiredSkillLevel_Execute.
fn set_word(e: &mut Engine, w: &[u32], field: Field<TESRecipe, u32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(r) = recipe_arg(e, &read) {
        e.set(r, field, read.int(1, 0));
    }
    handled(true)
}

fn get_recipe_required_skill(e: &mut Engine, w: &[u32]) -> Ret {
    get_word(e, w, TESRecipe::reqSkill)
}
fn set_recipe_required_skill(e: &mut Engine, w: &[u32]) -> Ret {
    set_word(e, w, TESRecipe::reqSkill)
}
fn get_recipe_required_skill_level(e: &mut Engine, w: &[u32]) -> Ret {
    get_word(e, w, TESRecipe::reqSkillLevel)
}
fn set_recipe_required_skill_level(e: &mut Engine, w: &[u32]) -> Ret {
    set_word(e, w, TESRecipe::reqSkillLevel)
}

// ---- category ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_recipe.h:
// Cmd_GetRecipeCategory_Execute and Cmd_GetRecipeSubcategory_Execute.
fn get_category(e: &mut Engine, w: &[u32], field: Field<TESRecipe, u32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(r) = recipe_arg(e, &read) {
        let category = e.get(r, field);
        if category != 0 {
            set_form_result(e, &a, category);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_recipe.h:
// Cmd_SetRecipeCategory_Execute and Cmd_SetRecipeSubcategory_Execute.
/// A category (`IS_ID` 0x6B) into the pointer and its id into the id word.
fn set_category(
    e: &mut Engine,
    w: &[u32],
    field: Field<TESRecipe, u32>,
    id_field: Field<TESRecipe, u32>,
) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let category = read.form(1);
    if let Some(r) = recipe_arg(e, &read).filter(|_| is_category(e, category)) {
        e.set(r, field, category);
        let id = super::ref_id(e, category);
        e.set(r, id_field, id);
    }
    handled(true)
}

fn get_recipe_category(e: &mut Engine, w: &[u32]) -> Ret {
    get_category(e, w, TESRecipe::category)
}
fn set_recipe_category(e: &mut Engine, w: &[u32]) -> Ret {
    set_category(e, w, TESRecipe::category, TESRecipe::categoryID)
}
fn get_recipe_subcategory(e: &mut Engine, w: &[u32]) -> Ret {
    get_category(e, w, TESRecipe::subCategory)
}
fn set_recipe_subcategory(e: &mut Engine, w: &[u32]) -> Ret {
    set_category(e, w, TESRecipe::subCategory, TESRecipe::subCategoryID)
}

// ---- components ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp:
// TESRecipe::ComponentList::GetComponents.
/// An array of the items of the list's components (an empty node skipped, a null item as form
/// 0), made for any recipe even when empty.
fn get_forms(e: &mut Engine, w: &[u32], outputs: bool) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(r) = recipe_arg(e, &read) {
        let mut tmp = TempElements::new();
        for c in list_items(e, components(r, outputs)) {
            if c != 0 {
                tmp.append(Element::Form(item(e, c)));
            }
        }
        let id = create_array(e, a.script, &tmp.0);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

fn get_recipe_input_forms(e: &mut Engine, w: &[u32]) -> Ret {
    get_forms(e, w, false)
}
fn get_recipe_output_forms(e: &mut Engine, w: &[u32]) -> Ret {
    get_forms(e, w, true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp:
// TESRecipe::ComponentList::AddComponent.
/// A new component (8 bytes of the game's heap: the count, the form) put first.
fn add_form(e: &mut Engine, w: &[u32], outputs: bool) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(r) = recipe_arg(e, &read) {
        let (form, count) = (read.form(1), read.int(2, 0));
        let c = Ptr::<RecipeComponent>::new(heap_alloc(e, RecipeComponent::SIZE));
        e.set(c, RecipeComponent::quantity, count);
        e.set(c, RecipeComponent::item, form);
        list_prepend(e, components(r, outputs), c.addr());
    }
    handled(true)
}

fn add_recipe_input_form(e: &mut Engine, w: &[u32]) -> Ret {
    add_form(e, w, false)
}
fn add_recipe_output_form(e: &mut Engine, w: &[u32]) -> Ret {
    add_form(e, w, true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp:
// TESRecipe::ComponentList::RemoveComponent.
/// The first component holding the form: its count the result, the component freed, then its
/// node taken out (a later node freed; the first takes the next one's item). JIP reads the item
/// of an empty node through null; the port skips empty nodes.
fn remove_form(e: &mut Engine, w: &[u32], outputs: bool) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(r) = recipe_arg(e, &read) {
        let form = read.form(1);
        let (mut node, mut prev) = (components(r, outputs), Ptr::<ListNode>::NULL);
        let mut count = 0;
        loop {
            let c = e.get(node, ListNode::data);
            if c != 0 && item(e, c) == form {
                count = e.get(Ptr::<RecipeComponent>::new(c), RecipeComponent::quantity);
                heap_free(e, c);
                list_unlink(e, node, prev);
                break;
            }
            prev = node;
            node = Ptr::new(e.get(node, ListNode::next));
            if node.is_null() {
                break;
            }
        }
        a.set_result(e, f64::from(count as i32));
    }
    handled(true)
}

fn remove_recipe_input_form(e: &mut Engine, w: &[u32]) -> Ret {
    remove_form(e, w, false)
}
fn remove_recipe_output_form(e: &mut Engine, w: &[u32]) -> Ret {
    remove_form(e, w, true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp:
// TESRecipe::ComponentList::ReplaceComponent.
/// The first component holding the form gets the other form.
fn replace_form(e: &mut Engine, w: &[u32], outputs: bool) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(r) = recipe_arg(e, &read) {
        let c = find_component(e, components(r, outputs), read.form(1));
        if c != 0 {
            e.set(
                Ptr::<RecipeComponent>::new(c),
                RecipeComponent::item,
                read.form(2),
            );
        }
    }
    handled(true)
}

fn replace_recipe_input_form(e: &mut Engine, w: &[u32]) -> Ret {
    replace_form(e, w, false)
}
fn replace_recipe_output_form(e: &mut Engine, w: &[u32]) -> Ret {
    replace_form(e, w, true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp:
// TESRecipe::ComponentList::GetQuantity.
/// The count of the first component holding the form, as an `int` (0 without one).
fn get_count(e: &mut Engine, w: &[u32], outputs: bool) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(r) = recipe_arg(e, &read) {
        let c = find_component(e, components(r, outputs), read.form(1));
        let count = if c == 0 {
            0
        } else {
            e.get(Ptr::<RecipeComponent>::new(c), RecipeComponent::quantity)
        };
        a.set_result(e, f64::from(count as i32));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp:
// TESRecipe::ComponentList::SetQuantity.
fn set_count(e: &mut Engine, w: &[u32], outputs: bool) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(r) = recipe_arg(e, &read) {
        let c = find_component(e, components(r, outputs), read.form(1));
        if c != 0 {
            e.set(
                Ptr::<RecipeComponent>::new(c),
                RecipeComponent::quantity,
                read.int(2, 0),
            );
        }
    }
    handled(true)
}

fn get_recipe_input_count(e: &mut Engine, w: &[u32]) -> Ret {
    get_count(e, w, false)
}
fn set_recipe_input_count(e: &mut Engine, w: &[u32]) -> Ret {
    set_count(e, w, false)
}
fn get_recipe_output_count(e: &mut Engine, w: &[u32]) -> Ret {
    get_count(e, w, true)
}
fn set_recipe_output_count(e: &mut Engine, w: &[u32]) -> Ret {
    set_count(e, w, true)
}

// ---- the recipes of a form ----

/// The data handler's recipes in order (`g_dataHandler->recipeList`), empty nodes skipped, and
/// only those of the category when one is given (a non-category filter counts as none).
fn recipes(e: &Engine, filter: u32) -> Vec<Ptr<TESRecipe>> {
    let filter = if filter != 0 && is_category(e, filter) {
        filter
    } else {
        0
    };
    let dh = Ptr::<DataHandler>::new(e.mem.u32(DATA_HANDLER));
    list_items(e, dh.at(DataHandler::recipeList))
        .into_iter()
        .filter(|&r| r != 0)
        .map(Ptr::<TESRecipe>::new)
        .filter(|&r| {
            filter == 0
                || e.get(r, TESRecipe::category) == filter
                || e.get(r, TESRecipe::subCategory) == filter
        })
        .collect()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_recipe.h:
// Cmd_GetFormRecipes_Execute.
/// An array of the recipes with the form among their inputs, a recipe once per such input; made
/// even when empty (nothing when the read fails).
fn get_form_recipes(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (form, filter) = (read.form(0), read.form(1));
    let mut tmp = TempElements::new();
    for r in recipes(e, filter) {
        for c in list_items(e, r.at(TESRecipe::inputs)) {
            if c != 0 && item(e, c) == form {
                tmp.append(Element::Form(r.addr()));
            }
        }
    }
    let id = create_array(e, a.script, &tmp.0);
    a.set_result(e, f64::from(id));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_recipe.h:
// Cmd_GetFormRecipeOutputs_Execute.
/// An array of the output items of the recipes with the form among their inputs, each item once
/// (sorted by address, `InsertUnique`); no array when there are none.
fn get_form_recipe_outputs(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (form, filter) = (read.form(0), read.form(1));
    let mut tmp = TempElements::new();
    for r in recipes(e, filter) {
        for c in list_items(e, r.at(TESRecipe::inputs)) {
            if c == 0 || item(e, c) != form {
                continue;
            }
            for o in list_items(e, r.at(TESRecipe::outputs)) {
                if o != 0 {
                    tmp.insert_unique(Element::Form(item(e, o)));
                }
            }
        }
    }
    if !tmp.is_empty() {
        let id = create_array(e, a.script, &tmp.0);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

// ---- conditions ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_recipe.h:
// Cmd_AddRecipeCondition_Execute.
/// A new condition (0x1C bytes of the game's heap: the comparison as its type byte, the value,
/// the function, the form as its first parameter, the rest 0; the three pad bytes left as the
/// heap gave them) appended to the recipe's conditions.
fn add_recipe_condition(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(r) = recipe_arg(e, &read) else {
        return handled(true);
    };
    let (func, param, comp, value) = (
        read.int(1, 0),
        read.form(2),
        read.int(3, 0),
        read.float(4, 0.0),
    );
    let c = Ptr::<Condition>::new(heap_alloc(e, Condition::SIZE));
    e.set(c, Condition::cType, comp as u8);
    e.set(c, Condition::comparisonValue, value);
    e.set(c, Condition::opcode, func);
    e.set(c, Condition::parameter1, param);
    e.set(c, Condition::parameter2, 0);
    e.set(c, Condition::runOnType, 0);
    e.set(c, Condition::reference, 0);
    list_append(e, r.at(TESRecipe::conditions), c.addr());
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_recipe_tests.rs"]
mod tests;
