//! Verificação da hierarquia de níveis (seção 6 da especificação): quem é
//! filho de quem, com uma pilha. Isto não está na gramática de `grammar.rs`
//! porque depende de contexto — dois `entrada` seguidos são sintaticamente
//! idênticos, sejam eles irmãos ou pai e filho — e por isso roda depois,
//! num segundo passo, quando todos os símbolos já foram montados (D23).

use std::collections::HashMap;

use crate::symbols::SymbolKind;

use super::{ParseError, Parser};

impl<'t> Parser<'t> {
    /// Verifica a hierarquia de níveis: quem é filho de quem, se um `77`
    /// não está subordinado a ninguém, se todo item sem PIC acabou virando
    /// grupo (teve subordinados) e se todo item 02-49 tem um pai. No meio
    /// do processo, chama [`Parser::check_redefines`] (definida em
    /// `redefines.rs`) — a validação de `REDEFINES` depende dos pais já
    /// estarem resolvidos. Por último, recalcula o tamanho de cada grupo,
    /// já considerando a regra de `REDEFINES` (seção 6.1).
    pub(super) fn check_hierarchy(&mut self) {
        struct Aberto {
            level: u32,
            name: String,
        }
        let mut pilha: Vec<Aberto> = Vec::new();
        let mut tem_filhos: HashMap<String, bool> = HashMap::new();
        let mut pais: HashMap<String, Option<String>> = HashMap::new();

        for symbol in &self.symbols {
            if symbol.level == 77 {
                pilha.clear();
                pais.insert(symbol.name.clone(), None);
            } else {
                while pilha.last().is_some_and(|topo| topo.level >= symbol.level) {
                    pilha.pop();
                }
                let pai = pilha.last().map(|topo| topo.name.clone());
                if pai.is_none() && symbol.level != 1 {
                    // Acesso direto a `self.errors` (e não a um método),
                    // porque este laço já empresta `self.symbols` — um
                    // método pediria o `self` inteiro emprestado de novo.
                    self.errors.push(ParseError {
                        message: format!(
                            "nível {:02} de '{}' precisa estar subordinado a um item de nível \
                             menor, e não há nenhum aberto nesse ponto",
                            symbol.level, symbol.name
                        ),
                        line: symbol.line,
                        col: 1,
                    });
                }
                if let Some(p) = &pai {
                    tem_filhos.insert(p.clone(), true);
                }
                pais.insert(symbol.name.clone(), pai);
                if symbol.pic.is_none() {
                    pilha.push(Aberto { level: symbol.level, name: symbol.name.clone() });
                }
            }
        }

        for symbol in &mut self.symbols {
            symbol.parent = pais.get(&symbol.name).cloned().flatten();
            if symbol.pic.is_none() && !tem_filhos.get(&symbol.name).copied().unwrap_or(false) {
                let motivo = if symbol.level == 77 {
                    "o nível 77 sempre exige a cláusula PIC".to_string()
                } else {
                    "faltou a cláusula PIC, e o item também não tem nenhum subordinado \
                     (precisa ser um ou outro)"
                        .to_string()
                };
                self.errors.push(ParseError {
                    message: format!("item '{}' incompleto: {motivo}", symbol.name),
                    line: symbol.line,
                    col: 1,
                });
            }
        }

        self.check_redefines();

        // Recalcula o tamanho dos grupos como a soma dos filhos diretos,
        // agora que todo pai já está resolvido (D21). Um item que redefine
        // outro (D24) não soma espaço novo: ele já está contado dentro do
        // item que redefine, como o maior tamanho entre os dois (D25).
        let mut maior_redefinicao: HashMap<String, usize> = HashMap::new();
        for symbol in &self.symbols {
            if let Some(base) = &symbol.redefines {
                let atual = maior_redefinicao.entry(base.clone()).or_insert(0);
                *atual = (*atual).max(symbol.size_bytes);
            }
        }
        // Percorre de trás para frente (decisão D26): num arquivo COBOL, um
        // item sempre é declarado depois do grupo que o contém — inclusive
        // um grupo aninhado dentro de outro grupo (ex.: CONTA dentro de
        // CLIENTE). Andando de trás pra frente, ao chegar na linha de um
        // grupo, todo filho dele já teve o tamanho resolvido nesta mesma
        // passada, mesmo quando esse filho é ele próprio um grupo. Numa
        // única passada para a frente, um grupo aninhado ficava de fora da
        // soma do grupo mais externo, porque o tamanho dele só existia
        // depois de todo o cálculo terminar.
        let mut tamanhos: HashMap<String, usize> = HashMap::new();
        for i in (0..self.symbols.len()).rev() {
            if matches!(self.symbols[i].kind, SymbolKind::Group) {
                self.symbols[i].size_bytes = *tamanhos.get(&self.symbols[i].name).unwrap_or(&0);
            }
            let symbol = &self.symbols[i];
            if symbol.redefines.is_none() {
                if let Some(pai) = &symbol.parent {
                    let tamanho = symbol.size_bytes.max(*maior_redefinicao.get(&symbol.name).unwrap_or(&0));
                    *tamanhos.entry(pai.clone()).or_insert(0) += tamanho;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse_source;

    #[test]
    fn nivel_77_nao_pode_ter_subordinado() {
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n77 CONTADOR PIC S9(4).\n05 FILHO PIC X.\n";
        let (_, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("precisa estar subordinado"));
    }

    /// Regressão: um grupo dentro de outro grupo (ex.: CONTA dentro de
    /// CLIENTE) tem que contar no tamanho do grupo de fora. O cálculo antigo
    /// (uma soma só, ignorando itens que também eram grupo) deixava CONTA
    /// de fora da soma de CLIENTE.
    #[test]
    fn grupo_aninhado_conta_no_tamanho_do_grupo_de_fora() {
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n\
            01 CLIENTE.\n\
            \x20\x2005 NOME  PIC X(30).\n\
            \x20\x2005 CONTA.\n\
            \x20\x20\x20\x2010 SALDO  PIC S9(7)V99.\n\
            \x20\x20\x20\x2010 LIMITE PIC S9(7)V99.\n";
        let (symbols, errors) = parse_source(fonte);
        assert!(errors.is_empty(), "erros inesperados: {errors:?}");

        let conta = symbols.iter().find(|s| s.name == "CONTA").unwrap();
        assert_eq!(conta.size_bytes, 9 + 9); // SALDO + LIMITE

        let cliente = symbols.iter().find(|s| s.name == "CLIENTE").unwrap();
        assert_eq!(cliente.size_bytes, 30 + 9 + 9); // NOME + CONTA (SALDO+LIMITE)
    }

    #[test]
    fn item_sem_pic_e_sem_subordinados_e_erro() {
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n01 CLIENTE.\n";
        let (_, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("incompleto"), "{:?}", errors[0]);
    }
}
