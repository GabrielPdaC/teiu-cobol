//! Verificação da cláusula `REDEFINES` (seção 6.1 da especificação,
//! decisão D24): o alvo precisa existir, ser elementar (ter PIC), estar no
//! mesmo nível, e a entrada precisa vir logo depois do alvo — ou logo
//! depois de *outra* entrada que já redefina o mesmo alvo. É essa segunda
//! parte que permite `CPF` e `CNPJ` redefinirem os dois o mesmo
//! `CPF-CNPJ`, um depois do outro. Chamada por `hierarchy.rs`, depois que
//! os pais de cada item já foram resolvidos.

use super::{ParseError, Parser};

impl<'t> Parser<'t> {
    pub(super) fn check_redefines(&mut self) {
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

#[cfg(test)]
mod tests {
    use super::super::parse_source;
    use crate::symbols::SymbolKind;

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
