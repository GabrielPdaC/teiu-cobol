//! Tabela de símbolos e leitura de cadeias PIC (seção 7 da especificação).

/// Uma entrada da tabela de símbolos: um item declarado, já com o tamanho em
/// bytes calculado e o nome do pai na hierarquia (seção 6 da especificação).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub level: u32,
    pub kind: SymbolKind,
    /// Cadeia PIC como foi escrita, para exibição; `None` em itens de grupo.
    pub pic: Option<String>,
    pub size_bytes: usize,
    pub parent: Option<String>,
    /// Nome do item que este redefine (cláusula `REDEFINES`, decisão D24), ou
    /// nenhum se o item ocupa espaço próprio.
    pub redefines: Option<String>,
    pub line: usize,
}

/// Categoria do item, já traduzida para os três tipos do enunciado (seção 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    /// Item de grupo (sem PIC): o análogo da lista `int a, b, c;`.
    Group,
    /// `char`: `PIC X` ou `PIC X(n)`.
    Char { len: usize },
    /// `int`: `PIC 9(n)` ou `PIC S9(n)`.
    Int { signed: bool, len: usize },
    /// `float`: `PIC 9(n)V9(m)` ou `PIC S9(n)V9(m)`.
    Float { signed: bool, int_len: usize, frac_len: usize },
}

impl SymbolKind {
    /// Tamanho em bytes de um item elementar (D21: um byte por posição da
    /// cadeia PIC, sem representar aqui o byte extra que o sinal ocupa em
    /// formatos empacotados — está fora do escopo, seção 10, limitação L6).
    pub fn size_bytes(self) -> usize {
        match self {
            SymbolKind::Group => 0,
            SymbolKind::Char { len } => len,
            SymbolKind::Int { len, .. } => len,
            SymbolKind::Float { int_len, frac_len, .. } => int_len + frac_len,
        }
    }
}

/// Lê uma cadeia PIC já validada pelo léxico (nunca `InvalidPicString`) e
/// devolve a categoria correspondente. Implementa a leitura descrita na
/// seção 4.3 da especificação: se tem `X`, é `char`; senão é numérico, com
/// `V` separando a parte inteira da fracionária.
pub fn analyze_pic(pic: &str) -> SymbolKind {
    let upper = pic.to_ascii_uppercase();
    if upper.contains('X') {
        return SymbolKind::Char { len: count_runs(&upper, 'X') };
    }
    let signed = upper.starts_with('S');
    let body = if signed { &upper[1..] } else { upper.as_str() };
    match body.find('V') {
        Some(v) => SymbolKind::Float {
            signed,
            int_len: count_runs(&body[..v], '9'),
            frac_len: count_runs(&body[v + 1..], '9'),
        },
        None => SymbolKind::Int { signed, len: count_runs(body, '9') },
    }
}

/// Soma a contagem de cada repetição do caractere `letter` em `s`, incluindo
/// a forma `letter(n)` — por exemplo, `count_runs("9(05)9(3)", '9')` é 8.
fn count_runs(s: &str, letter: char) -> usize {
    let chars: Vec<char> = s.chars().collect();
    let mut total = 0;
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != letter {
            i += 1;
            continue;
        }
        i += 1;
        if chars.get(i) == Some(&'(') {
            let start = i + 1;
            let mut end = start;
            while chars.get(end).is_some_and(|c| *c != ')') {
                end += 1;
            }
            let count: String = chars[start..end].iter().collect();
            total += count.parse::<usize>().unwrap_or(1);
            i = end + 1;
        } else {
            total += 1;
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_simples_e_com_contagem() {
        assert_eq!(analyze_pic("X"), SymbolKind::Char { len: 1 });
        assert_eq!(analyze_pic("X(30)"), SymbolKind::Char { len: 30 });
        assert_eq!(analyze_pic("X(2)X(3)"), SymbolKind::Char { len: 5 });
    }

    #[test]
    fn inteiro_com_e_sem_sinal() {
        assert_eq!(analyze_pic("9(04)"), SymbolKind::Int { signed: false, len: 4 });
        assert_eq!(analyze_pic("S9(4)"), SymbolKind::Int { signed: true, len: 4 });
    }

    #[test]
    fn decimal_com_e_sem_sinal() {
        assert_eq!(
            analyze_pic("S9(7)V99"),
            SymbolKind::Float { signed: true, int_len: 7, frac_len: 2 }
        );
        assert_eq!(
            analyze_pic("9(08)v99"),
            SymbolKind::Float { signed: false, int_len: 8, frac_len: 2 }
        );
    }

    #[test]
    fn tamanho_em_bytes() {
        assert_eq!(SymbolKind::Char { len: 30 }.size_bytes(), 30);
        assert_eq!(SymbolKind::Int { signed: true, len: 4 }.size_bytes(), 4);
        assert_eq!(
            SymbolKind::Float { signed: true, int_len: 7, frac_len: 2 }.size_bytes(),
            9
        );
        assert_eq!(SymbolKind::Group.size_bytes(), 0);
    }
}
