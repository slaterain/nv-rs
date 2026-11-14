//! The passes over the syntax tree (`Compiler/Passes` (xNVSE)), in the order
//! `CompileNVSEScript` runs them: [`match_transform`], [`variables`], [`calls`], [`types`].

pub mod calls;
pub mod match_transform;
pub mod types;
pub mod variables;

use super::lexer::TokenType;
use crate::expr::op;

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Utils.cpp: tokenOpToNVSEOpType.
/// `tokenOpToNVSEOpType[type]`: the operator a token compiles to. Read through `operator[]`,
/// which answers `kOpType_Assignment` (0) for a token the map lacks (`++`, `--`).
pub fn token_op(t: TokenType) -> u8 {
    use TokenType as T;
    match t {
        T::EqEq => op::EQUALS,
        T::LogicOr => op::LOGICAL_OR,
        T::LogicAnd => op::LOGICAL_AND,
        T::Greater => op::GREATER_THAN,
        T::GreaterEq => op::GREATER_OR_EQUAL,
        T::Less => op::LESS_THAN,
        T::LessEq => op::LESS_OR_EQUAL,
        T::Bang => op::LOGICAL_NOT,
        T::BangEq => op::NOT_EQUAL,
        T::Plus => op::ADD,
        T::Minus => op::SUBTRACT,
        T::Star => op::MULTIPLY,
        T::Slash => op::DIVIDE,
        T::Mod => op::MODULO,
        T::Pow => op::EXPONENT,
        T::Eq => op::ASSIGNMENT,
        T::PlusEq => op::PLUS_EQUALS,
        T::MinusEq => op::MINUS_EQUALS,
        T::StarEq => op::TIMES_EQUALS,
        T::SlashEq => op::DIVIDED_EQUALS,
        T::ModEq => op::MODULO_EQUALS,
        T::PowEq => op::EXPONENT_EQUALS,
        T::MakePair => op::MAKE_PAIR,
        T::Slice => op::SLICE,
        T::BitwiseAnd => op::BITWISE_AND,
        T::BitwiseOr => op::BITWISE_OR,
        T::BitwiseAndEquals => op::BITWISE_AND_EQUALS,
        T::BitwiseOrEquals => op::BITWISE_OR_EQUALS,
        T::LeftShift => op::LEFT_SHIFT,
        T::RightShift => op::RIGHT_SHIFT,
        T::Negate => op::NEGATION,
        T::Dollar => op::TO_STRING,
        T::Pound => op::TO_NUMBER,
        T::Box => op::BOX,
        T::Unbox => op::DEREFERENCE,
        T::LeftBracket => op::LEFT_BRACKET,
        T::Dot => op::DOT,
        T::BitwiseNot => op::BITWISE_NOT,
        _ => op::ASSIGNMENT,
    }
}

/// Bytes joined (a message with a script's own text in it, kept as bytes).
pub fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}
