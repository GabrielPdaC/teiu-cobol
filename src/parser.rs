//! Analisador sintático descendente recursivo da gramática de declarações
//! (decisão D7) e verificação da hierarquia de níveis (seção 6 da
//! especificação). Consome os tokens de `lexer.rs` e produz a tabela de
//! símbolos (`symbols.rs`) e os erros de estrutura.
//!
//! Gramática (seção 5 da especificação):
//!
//! ```text
//! programa     = cabecalho , { entrada } ;
//! cabecalho    = "DATA" , "DIVISION" , "." , "WORKING-STORAGE" , "SECTION" , "." ;
//! entrada      = NIVEL , NOME , [ clausula_redefines ] , [ clausula_pic ] , "." ;
//! clausula_redefines = "REDEFINES" , NOME ;
//! clausula_pic = ( "PIC" | "PICTURE" ) , [ "IS" ] , CADEIA_PIC ;
//! ```
//!
//! A hierarquia de níveis (quem é filho de quem) não está na gramática acima
//! porque depende de contexto — dois `entrada` seguidos são sintaticamente
//! iguais, sejam eles irmãos ou pai e filho. Por isso é verificada à parte,
//! com uma pilha (seção 6).

use std::collections::HashMap;

use crate::lexer::{Token, TokenKind};
use crate::symbols::{analyze_pic, Symbol, SymbolKind};

/// Um erro de estrutura, sempre com linha e coluna (seção 9 do enunciado).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

/// Analisa a sequência de tokens e devolve a tabela de símbolos e os erros de
/// estrutura encontrados. Continua após cada erro (recuperação em modo
/// pânico: pula até o próximo ponto final) para relatar o máximo possível de
/// uma vez, como pede o enunciado.
pub fn parse(tokens: &[Token]) -> (Vec<Symbol>, Vec<ParseError>) {
    let mut parser = Parser { tokens, pos: 0, errors: Vec::new(), symbols: Vec::new() };
    parser.parse_header();
    while parser.peek().is_some() {
        parser.parse_entry();
    }
    parser.check_hierarchy();
    (parser.symbols, parser.errors)
}

/// Um item ainda sendo montado, antes de virar `Symbol` (falta calcular o
/// tamanho em bytes dos grupos, que depende dos filhos).
struct RawEntry {
    name: String,
    level: u32,
    pic: Option<String>,
    redefines: Option<String>,
    parent: Option<String>,
    line: usize,
}

struct Parser<'t> {
    tokens: &'t [Token],
    pos: usize,
    errors: Vec<ParseError>,
    symbols: Vec<Symbol>,
}

impl<'t> Parser<'t> {
    fn peek(&self) -> Option<&'t Token> {
        self.tokens.get(self.pos)
    }

    /// Posição a usar num erro quando a entrada acabou: a do último token
    /// lido, ou (1, 1) se o arquivo estiver vazio.
    fn eof_position(&self) -> (usize, usize) {
        self.tokens.last().map(|t| (t.line, t.col)).unwrap_or((1, 1))
    }

    fn error(&mut self, message: impl Into<String>, line: usize, col: usize) {
        self.errors.push(ParseError { message: message.into(), line, col });
    }

    /// Recuperação em modo pânico: avança até consumir o próximo `Period`,
    /// para a próxima `entrada` começar limpa (D22).
    fn recover_to_period(&mut self) {
        while let Some(token) = self.peek() {
            self.pos += 1;
            if token.kind == TokenKind::Period {
                break;
            }
        }
    }

    fn parse_header(&mut self) {
        const ESPERADO: [TokenKind; 6] = [
            TokenKind::Data,
            TokenKind::Division,
            TokenKind::Period,
            TokenKind::WorkingStorage,
            TokenKind::Section,
            TokenKind::Period,
        ];
        let start = self.pos;
        for kind in &ESPERADO {
            match self.peek() {
                Some(token) if &token.kind == kind => self.pos += 1,
                Some(token) => {
                    self.error(
                        format!(
                            "faltou o cabeçalho 'DATA DIVISION.' e 'WORKING-STORAGE SECTION.': \
                             encontrado {}",
                            describe(&token.kind)
                        ),
                        token.line,
                        token.col,
                    );
                    self.pos = start;
                    return;
                }
                None => {
                    let (line, col) = self.eof_position();
                    self.error("faltou o cabeçalho 'DATA DIVISION.' e 'WORKING-STORAGE SECTION.'", line, col);
                    self.pos = start;
                    return;
                }
            }
        }
    }

    fn parse_entry(&mut self) {
        let Some(level_token) = self.peek() else { return };
        let TokenKind::Number(level_text) = &level_token.kind else {
            self.error(
                format!(
                    "esperava um nível de declaração (01-49 ou 77), mas encontrou {}",
                    describe(&level_token.kind)
                ),
                level_token.line,
                level_token.col,
            );
            self.recover_to_period();
            return;
        };
        let level_text = level_text.clone();
        let (level_line, level_col) = (level_token.line, level_token.col);
        self.pos += 1;

        let level: u32 = match level_text.parse() {
            Ok(n) if n == 77 || (1..=49).contains(&n) => n,
            _ => {
                self.error(
                    format!("nível '{level_text}' inválido: use 01-49 ou 77"),
                    level_line,
                    level_col,
                );
                self.recover_to_period();
                return;
            }
        };

        let name = match self.peek() {
            Some(Token { kind: TokenKind::Name(name), .. }) => {
                let name = name.clone();
                self.pos += 1;
                name
            }
            Some(token) => {
                self.error(
                    format!(
                        "esperava um nome de dado depois do nível {level_text}, mas encontrou {}",
                        describe(&token.kind)
                    ),
                    token.line,
                    token.col,
                );
                self.recover_to_period();
                return;
            }
            None => {
                let (line, col) = self.eof_position();
                self.error(
                    format!("esperava um nome de dado depois do nível {level_text}"),
                    line,
                    col,
                );
                return;
            }
        };

        let redefines = match self.parse_redefines_clause(&name) {
            Ok(redefines) => redefines,
            Err(()) => return,
        };

        let pic = match self.parse_pic_clause(&name) {
            Ok(pic) => pic,
            Err(()) => return, // erro já registrado; recuperação já feita
        };

        if let Some(Token { kind: TokenKind::UnsupportedReserved(word), line, col }) = self.peek() {
            self.error(
                format!(
                    "cláusula '{word}' não é suportada nesta etapa: as variáveis são apenas \
                     declaradas, sem inicialização (decisão D5 da especificação)"
                ),
                *line,
                *col,
            );
            self.recover_to_period();
            return;
        }

        match self.peek() {
            Some(Token { kind: TokenKind::Period, .. }) => self.pos += 1,
            Some(token) => {
                self.error(
                    format!(
                        "declaração mal formada: esperava '.' depois de '{name}', mas encontrou {}",
                        describe(&token.kind)
                    ),
                    token.line,
                    token.col,
                );
                self.recover_to_period();
                return;
            }
            None => {
                let (line, col) = self.eof_position();
                self.error(format!("faltou o ponto final da declaração de '{name}'"), line, col);
                return;
            }
        }

        self.symbols.push(build_symbol(RawEntry {
            name,
            level,
            pic,
            redefines,
            parent: None, // preenchido em check_hierarchy
            line: level_line,
        }));
    }

    /// Lê `[ "REDEFINES" NOME ]`. Devolve `Ok(None)` quando não há cláusula
    /// `REDEFINES` (a maioria dos itens). A validação de que o alvo existe e
    /// vem no lugar certo (seção 6) fica para `check_hierarchy`, porque
    /// depende de itens que ainda não foram lidos neste ponto do arquivo.
    fn parse_redefines_clause(&mut self, name: &str) -> Result<Option<String>, ()> {
        if !matches!(self.peek(), Some(Token { kind: TokenKind::Redefines, .. })) {
            return Ok(None);
        }
        self.pos += 1;
        match self.peek() {
            Some(Token { kind: TokenKind::Name(alvo), .. }) => {
                let alvo = alvo.clone();
                self.pos += 1;
                Ok(Some(alvo))
            }
            Some(token) => {
                self.error(
                    format!(
                        "esperava o nome do item redefinido depois de 'REDEFINES' em '{name}', \
                         mas encontrou {}",
                        describe(&token.kind)
                    ),
                    token.line,
                    token.col,
                );
                self.recover_to_period();
                Err(())
            }
            None => {
                let (line, col) = self.eof_position();
                self.error(
                    format!("esperava o nome do item redefinido depois de 'REDEFINES' em '{name}'"),
                    line,
                    col,
                );
                Err(())
            }
        }
    }

    /// Lê `[ ("PIC"|"PICTURE") ["IS"] CADEIA_PIC ]`. Devolve `Ok(None)`
    /// quando não há cláusula PIC (item de grupo em potencial).
    fn parse_pic_clause(&mut self, name: &str) -> Result<Option<String>, ()> {
        if !matches!(self.peek(), Some(Token { kind: TokenKind::Pic, .. })) {
            return Ok(None);
        }
        self.pos += 1;
        if matches!(self.peek(), Some(Token { kind: TokenKind::Is, .. })) {
            self.pos += 1;
        }
        match self.peek() {
            Some(Token { kind: TokenKind::PicString(pic), .. }) => {
                let pic = pic.clone();
                self.pos += 1;
                Ok(Some(pic))
            }
            Some(token) => {
                self.error(
                    format!(
                        "esperava uma cadeia PIC depois de 'PIC' em '{name}', mas encontrou {}",
                        describe(&token.kind)
                    ),
                    token.line,
                    token.col,
                );
                self.recover_to_period();
                Err(())
            }
            None => {
                let (line, col) = self.eof_position();
                self.error(format!("esperava uma cadeia PIC depois de 'PIC' em '{name}'"), line, col);
                Err(())
            }
        }
    }

    /// Verifica a hierarquia de níveis (seção 6): quem é filho de quem, se
    /// um `77` não está subordinado a ninguém, se todo item sem PIC acabou
    /// virando grupo (teve subordinados) e se todo item 02-49 tem um pai.
    /// Como a montagem dos símbolos já terminou, isto é feito num segundo
    /// passo, e não durante `parse_entry` (D23).
    fn check_hierarchy(&mut self) {
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
                    // Acesso direto a `self.errors` (e não `self.error(...)`),
                    // porque este laço já empresta `self.symbols` — só um
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
        let tamanhos: HashMap<String, usize> = self
            .symbols
            .iter()
            .filter(|s| !matches!(s.kind, SymbolKind::Group) && s.redefines.is_none())
            .fold(HashMap::new(), |mut acc, s| {
                if let Some(pai) = &s.parent {
                    let tamanho = s.size_bytes.max(*maior_redefinicao.get(&s.name).unwrap_or(&0));
                    *acc.entry(pai.clone()).or_insert(0) += tamanho;
                }
                acc
            });
        for symbol in &mut self.symbols {
            if matches!(symbol.kind, SymbolKind::Group) {
                symbol.size_bytes = *tamanhos.get(&symbol.name).unwrap_or(&0);
            }
        }
    }

    /// Valida a cláusula `REDEFINES` (seção 6, decisão D24): o alvo precisa
    /// existir, ser elementar (ter PIC), estar no mesmo nível, e vir logo
    /// antes desta entrada — ou logo antes vem *outra* entrada que já
    /// redefine o mesmo alvo, o que permite `CPF` e `CNPJ` redefinirem os
    /// dois o mesmo `CPF-CNPJ`, um depois do outro.
    fn check_redefines(&mut self) {
        for i in 0..self.symbols.len() {
            let Some(alvo_nome) = self.symbols[i].redefines.clone() else { continue };
            let entrada = &self.symbols[i];
            let (nome, nivel, line) = (entrada.name.clone(), entrada.level, entrada.line);

            if entrada.pic.is_none() {
                self.errors.push(ParseError {
                    message: format!(
                        "'{nome}' não pode ter REDEFINES: nesta etapa, só itens elementares \
                         (com PIC) podem redefinir outro item"
                    ),
                    line,
                    col: 1,
                });
                continue;
            }

            let anterior = i.checked_sub(1).map(|j| &self.symbols[j]);
            let encadeia_do_mesmo_alvo =
                anterior.is_some_and(|a| a.name == alvo_nome || a.redefines.as_deref() == Some(&alvo_nome));

            if !encadeia_do_mesmo_alvo {
                self.errors.push(ParseError {
                    message: format!(
                        "REDEFINES de '{nome}' deve vir logo depois de '{alvo_nome}', ou de \
                         outro item que já redefina '{alvo_nome}' — é assim que CPF e CNPJ, por \
                         exemplo, podem redefinir o mesmo campo"
                    ),
                    line,
                    col: 1,
                });
                continue;
            }

            let anterior = anterior.expect("encadeia_do_mesmo_alvo garante que existe");
            if anterior.level != nivel {
                self.errors.push(ParseError {
                    message: format!(
                        "REDEFINES de '{nome}' está no nível {nivel:02}, mas '{}' está no nível \
                         {:02}; os dois precisam ter o mesmo nível",
                        anterior.name, anterior.level
                    ),
                    line,
                    col: 1,
                });
            } else if anterior.pic.is_none() {
                self.errors.push(ParseError {
                    message: format!(
                        "'{nome}' não pode redefinir '{alvo_nome}': nesta etapa, só itens \
                         elementares (com PIC) podem ser redefinidos"
                    ),
                    line,
                    col: 1,
                });
            }
        }
    }
}

fn build_symbol(entry: RawEntry) -> Symbol {
    let kind = match &entry.pic {
        Some(pic) => analyze_pic(pic),
        None => SymbolKind::Group,
    };
    Symbol {
        name: entry.name,
        level: entry.level,
        kind,
        size_bytes: kind.size_bytes(),
        pic: entry.pic,
        parent: entry.parent,
        redefines: entry.redefines,
        line: entry.line,
    }
}

/// Descreve um token para mensagens de erro em português (seção 9).
fn describe(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Data => "a palavra reservada 'DATA'".into(),
        TokenKind::Division => "a palavra reservada 'DIVISION'".into(),
        TokenKind::WorkingStorage => "a palavra reservada 'WORKING-STORAGE'".into(),
        TokenKind::Section => "a palavra reservada 'SECTION'".into(),
        TokenKind::Pic => "a palavra reservada 'PIC'".into(),
        TokenKind::Is => "a palavra reservada 'IS'".into(),
        TokenKind::Redefines => "a palavra reservada 'REDEFINES'".into(),
        TokenKind::UnsupportedReserved(w) => format!("a palavra reservada '{w}'"),
        TokenKind::Number(n) => format!("o número '{n}'"),
        TokenKind::Name(n) => format!("o nome '{n}'"),
        TokenKind::Period => "o ponto final".into(),
        TokenKind::PicString(s) => format!("a cadeia PIC '{s}'"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn parse_source(source: &[u8]) -> (Vec<Symbol>, Vec<ParseError>) {
        let (tokens, lex_errors) = lex(source);
        assert!(lex_errors.is_empty(), "fonte não deveria ter erro léxico: {lex_errors:?}");
        parse(&tokens)
    }

    #[test]
    fn programa_valido_com_grupo_e_item_isolado() {
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n\
            77 CONTADOR PIC S9(4).\n01 CLIENTE.\n   05 NOME PIC X(30).\n   05 SALDO PIC S9(7)V99.\n";
        let (symbols, errors) = parse_source(fonte);
        assert!(errors.is_empty(), "erros inesperados: {errors:?}");
        assert_eq!(symbols.len(), 4);

        let contador = &symbols[0];
        assert_eq!(contador.name, "CONTADOR");
        assert_eq!(contador.kind, SymbolKind::Int { signed: true, len: 4 });
        assert_eq!(contador.parent, None);

        let cliente = &symbols[1];
        assert_eq!(cliente.kind, SymbolKind::Group);
        assert_eq!(cliente.size_bytes, 30 + 9); // NOME + SALDO

        let nome = &symbols[2];
        assert_eq!(nome.parent.as_deref(), Some("CLIENTE"));
        assert_eq!(nome.kind, SymbolKind::Char { len: 30 });

        let saldo = &symbols[3];
        assert_eq!(saldo.kind, SymbolKind::Float { signed: true, int_len: 7, frac_len: 2 });
    }

    #[test]
    fn falta_o_cabecalho() {
        let (_, errors) = parse_source(b"01 CLIENTE PIC X.\n");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("faltou o cabeçalho"));
    }

    #[test]
    fn dois_nomes_seguidos_e_erro_de_estrutura() {
        // Análogo a "int a b;" do enunciado.
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n01 A B PIC X.\n";
        let (_, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("esperava '.' depois de 'A'"), "{:?}", errors[0]);
    }

    #[test]
    fn falta_o_ponto_final() {
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n01 CLIENTE PIC X(10)\n";
        let (_, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("faltou o ponto final"));
    }

    #[test]
    fn value_e_erro_de_estrutura_nao_lexico() {
        // Literais entre aspas não fazem parte do subconjunto (limitação L2);
        // por isso o teste usa um valor numérico, para isolar o erro de
        // estrutura que a cláusula VALUE em si já basta para gerar.
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n01 W78-NUMPRG VALUE 1.\n";
        let (_, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("cláusula 'VALUE' não é suportada"));
    }

    #[test]
    fn nivel_77_nao_pode_ter_subordinado() {
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n77 CONTADOR PIC S9(4).\n05 FILHO PIC X.\n";
        let (_, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("precisa estar subordinado"));
    }

    #[test]
    fn item_sem_pic_e_sem_subordinados_e_erro() {
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n01 CLIENTE.\n";
        let (_, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("incompleto"), "{:?}", errors[0]);
    }

    #[test]
    fn nivel_fora_da_faixa_e_erro() {
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n88 X PIC X.\n";
        let (_, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("nível '88' inválido"));
    }

    #[test]
    fn recupera_apos_erro_e_relata_o_resto() {
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n01 A B PIC X.\n77 CONTADOR PIC 9(4).\n";
        let (symbols, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 1, "só o primeiro item tem erro: {errors:?}");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "CONTADOR");
    }

    // --- REDEFINES (decisão D24): cpf/cnpj redefinindo o mesmo campo ---

    const CPF_CNPJ: &[u8] = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n\
        01 PESSOA.\n\
        \x20\x2005 CPF-CNPJ PIC X(14).\n\
        \x20\x2005 CPF       REDEFINES CPF-CNPJ PIC 9(11).\n\
        \x20\x2005 CNPJ      REDEFINES CPF-CNPJ PIC 9(14).\n";

    #[test]
    fn cpf_e_cnpj_redefinem_o_mesmo_campo() {
        let (symbols, errors) = parse_source(CPF_CNPJ);
        assert!(errors.is_empty(), "erros inesperados: {errors:?}");
        assert_eq!(symbols.len(), 4);

        let pessoa = &symbols[0];
        assert_eq!(pessoa.kind, SymbolKind::Group);
        // CPF-CNPJ (14) é o maior entre ele mesmo e as duas redefinições
        // (11 e 14); CPF e CNPJ não somam espaço novo.
        assert_eq!(pessoa.size_bytes, 14);

        let cpf = &symbols[2];
        assert_eq!(cpf.name, "CPF");
        assert_eq!(cpf.redefines.as_deref(), Some("CPF-CNPJ"));
        assert_eq!(cpf.parent.as_deref(), Some("PESSOA"));

        let cnpj = &symbols[3];
        assert_eq!(cnpj.name, "CNPJ");
        // CNPJ referencia CPF-CNPJ (o alvo original), embora quem venha
        // logo antes na declaração seja CPF, não CPF-CNPJ.
        assert_eq!(cnpj.redefines.as_deref(), Some("CPF-CNPJ"));
        assert_eq!(cnpj.parent.as_deref(), Some("PESSOA"));
    }

    #[test]
    fn redefines_precisa_vir_logo_apos_o_alvo_ou_outra_redefinicao() {
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n\
            01 PESSOA.\n\
            \x20\x2005 CPF-CNPJ PIC X(14).\n\
            \x20\x2005 NOME      PIC X(30).\n\
            \x20\x2005 CPF       REDEFINES CPF-CNPJ PIC 9(11).\n";
        let (_, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("deve vir logo depois de 'CPF-CNPJ'"), "{:?}", errors[0]);
    }

    #[test]
    fn redefines_de_alvo_inexistente_e_erro() {
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n\
            01 PESSOA.\n\
            \x20\x2005 CPF REDEFINES NAO-EXISTE PIC 9(11).\n";
        let (_, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("deve vir logo depois de 'NAO-EXISTE'"));
    }

    #[test]
    fn redefines_com_nivel_diferente_e_erro() {
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n\
            01 PESSOA.\n\
            \x20\x2005 CPF-CNPJ  PIC X(14).\n\
            \x20\x2010 CPF       REDEFINES CPF-CNPJ PIC 9(11).\n";
        let (_, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("os dois precisam ter o mesmo nível"), "{:?}", errors[0]);
    }

    #[test]
    fn redefines_de_item_de_grupo_e_erro() {
        // DADOS não tem PIC (é um grupo em potencial); como CPF vem logo
        // depois e o "redefine", DADOS nunca ganha um subordinado — daí os
        // dois erros: DADOS ficou incompleto, e grupo não pode ser redefinido.
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n\
            01 PESSOA.\n\
            \x20\x2005 DADOS.\n\
            \x20\x2005 CPF REDEFINES DADOS PIC 9(11).\n";
        let (_, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors.iter().any(|e| e.message.contains("item 'DADOS' incompleto")));
        assert!(errors.iter().any(|e| e.message.contains("só itens elementares (com PIC) podem ser redefinidos")));
    }
}
