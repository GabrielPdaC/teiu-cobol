//! Tokens do analisador léxico, gerados pela crate `logos` (decisão D6 em
//! `docs/especificacao.md`). A entrada é lida como bytes (`&[u8]`), e não como
//! `&str`, por causa da decisão D8: um byte fora do ASCII deve virar um erro
//! léxico com linha e coluna, em vez de impedir a leitura do arquivo.
//!
//! Dois arquivos, um por modo — existem dois conjuntos de tokens porque a
//! cláusula PIC tem sua própria semântica léxica (decisão D15, "modo PIC"):
//! a mesma palavra (`X`, `9`) é uma cadeia PIC dentro da cláusula e um token
//! inválido fora dela. Isso é o equivalente, em `logos`, das *start
//! conditions* do Flex — o léxico troca de [`NormalToken`] para [`PicToken`]
//! com `Lexer::morph` ao reconhecer `Pic`, e volta ao reconhecer
//! `PicString`, `InvalidPicString` ou `Period` (ver `crate::lexer`).
//!
//! Os nomes das variantes seguem a tabela 4 de `docs/especificacao.md`.

mod normal;
mod pic;

pub use normal::NormalToken;
pub use pic::PicToken;
