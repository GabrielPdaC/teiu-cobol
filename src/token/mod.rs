//! Tokens do analisador léxico, gerados pela crate `logos`. O arquivo-fonte
//! é lido como bytes (`&[u8]`), não como `&str`, para que um byte fora do
//! ASCII vire erro léxico em vez de impedir a leitura do arquivo inteiro.
//!
//! Dois arquivos, um por modo do léxico: dentro de uma cláusula `PIC`, as
//! palavras `X` e `9` são símbolos de tipo; fora dela, não significam
//! nada. Por isso são dois `enum` de token diferentes — [`NormalToken`]
//! para o corpo do programa, [`PicToken`] para dentro da cláusula PIC — e
//! `crate::lexer` troca de um para o outro com `Lexer::morph` ao entrar e
//! sair da cláusula.

mod normal;
mod pic;

pub use normal::NormalToken;
pub use pic::PicToken;
