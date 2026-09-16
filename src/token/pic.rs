//! Tokens reconhecidos dentro da cláusula PIC (modo PIC).

use logos::Logos;

/// Tokens reconhecidos dentro da cláusula PIC (modo PIC).
#[derive(Logos, Debug, Clone, PartialEq, Eq)]
#[logos(utf8 = false)]
#[logos(skip r"[ \t\r\n]+")]
#[logos(skip(r"\*>[^\n]*", allow_greedy = true))]
pub enum PicToken {
    #[regex("IS", priority = 4, ignore(case))]
    Is,

    /// Cadeia PIC do subconjunto "espelho estrito" (decisão D4, D16): `X`
    /// (char), `9`/`S9` (int) ou `9V9`/`S9V9` (float), sem misturar `X` e `9`.
    /// `\(0*[1-9][0-9]*\)` é a subexpressão `COUNT` da seção 4.2 da
    /// especificação: a contagem entre parênteses, como em `X(30)`; não aceita
    /// `(0)`, mas aceita zeros à esquerda, como em `9(05)`.
    #[regex(
        r"(X(\(0*[1-9][0-9]*\))?)+|S?((9(\(0*[1-9][0-9]*\))?)+(V(9(\(0*[1-9][0-9]*\))?)*)?|V(9(\(0*[1-9][0-9]*\))?)+)",
        |lex| lex.slice().to_owned(),
        priority = 3,
        ignore(case)
    )]
    PicString(Vec<u8>),

    #[regex(r"\.", priority = 2)]
    Period,

    /// Regra pega-tudo de erro do modo PIC, análoga a `NormalToken::InvalidWord`.
    #[regex(r"[^ \t\r\n]*[^ \t\r\n.]", |lex| lex.slice().to_owned(), priority = 1)]
    InvalidPicString(Vec<u8>),
}
