//! Traduz uma posição em bytes (o que o `logos` devolve em `Span::start`)
//! para linha e coluna, ambas começando em 1.

pub(super) struct LineIndex {
    /// Deslocamento em bytes de onde cada linha começa; `starts[0]` é sempre 0.
    starts: Vec<usize>,
}

impl LineIndex {
    pub(super) fn new(source: &[u8]) -> Self {
        let mut starts = vec![0];
        starts.extend(source.iter().enumerate().filter(|&(_, &b)| b == b'\n').map(|(i, _)| i + 1));
        LineIndex { starts }
    }

    pub(super) fn locate(&self, offset: usize) -> (usize, usize) {
        let line_index = match self.starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        (line_index + 1, offset - self.starts[line_index] + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::LineIndex;
    use super::super::{lex, TokenKind};

    /// Teste direto de `LineIndex`, sem passar pelo léxico inteiro: linha 1
    /// vai de 0 a 4 ("DATA\n"), linha 2 começa em 5.
    #[test]
    fn locate_acha_a_linha_e_a_coluna_certas() {
        let indice = LineIndex::new(b"DATA\nDIVISION.\n");
        assert_eq!(indice.locate(0), (1, 1)); // "D" de DATA
        assert_eq!(indice.locate(4), (1, 5)); // o '\n' ainda conta como linha 1
        assert_eq!(indice.locate(5), (2, 1)); // "D" de DIVISION
        assert_eq!(indice.locate(9), (2, 5)); // "I" de DIVISION (5+4)
    }

    /// Teste de ponta a ponta: confere que `lex` usa `LineIndex` corretamente
    /// num arquivo com várias linhas e recuo.
    #[test]
    fn linha_e_coluna_em_fonte_com_varias_linhas() {
        let fonte = b"DATA DIVISION.\n01 CLIENTE.\n   05 NOME PIC X.\n";
        let (tokens, _) = lex(fonte);
        // "05" está na linha 3, coluna 4 (após três espaços).
        let numero_05 = tokens.iter().find(|t| matches!(&t.kind, TokenKind::Number(n) if n == "05"));
        assert_eq!(numero_05.map(|t| (t.line, t.col)), Some((3, 4)));
    }
}
