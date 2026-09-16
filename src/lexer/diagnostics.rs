//! Mensagens de erro léxico: o diagnóstico da regra pega-tudo `InvalidWord`
//! e a conversão do lexema (bytes) para texto.

/// Diagnostica uma palavra rejeitada pela regra pega-tudo `InvalidWord`,
/// testando as causas nesta ordem: caractere não permitido, depois hífen
/// nas pontas, depois falta de letra. A ordem importa porque uma mesma
/// palavra pode cair em mais de uma causa (ex.: "-2" não tem letra e
/// começa com hífen); só a primeira é reportada.
pub(super) fn diagnose_invalid_word(palavra: &str) -> String {
    const PERMITIDOS: &str = "letras, dígitos e hífen";

    if let Some(c) = palavra.chars().find(|c| !(c.is_ascii_alphanumeric() || *c == '-')) {
        return format!(
            "palavra inválida '{palavra}': caractere '{c}' não permitido (esperado {PERMITIDOS})"
        );
    }
    if palavra.starts_with('-') || palavra.ends_with('-') {
        return format!("palavra inválida '{palavra}': nome não pode começar nem terminar com hífen");
    }
    if !palavra.chars().any(|c| c.is_ascii_alphabetic()) {
        return format!("palavra inválida '{palavra}': nome precisa de pelo menos uma letra");
    }
    format!("palavra inválida '{palavra}'")
}

/// Converte o lexema (bytes) para exibição. O alfabeto da linguagem é ASCII;
/// um byte fora do ASCII só chega até aqui em casos de borda, e é
/// substituído pelo caractere de substituição do Unicode.
pub(super) fn to_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::super::lex;

    #[test]
    fn nome_nao_pode_comecar_com_hifen() {
        let (_, errors) = lex(b"-CONTA");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("não pode começar nem terminar com hífen"));
    }

    #[test]
    fn nome_nao_pode_terminar_com_hifen() {
        let (_, errors) = lex(b"CONTA-.");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("não pode começar nem terminar com hífen"));
    }

    #[test]
    fn caractere_nao_permitido_e_relatado() {
        let (_, errors) = lex(b"CONTA@X.");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("caractere '@' não permitido"), "{:?}", errors[0]);
    }

    #[test]
    fn palavra_sem_letra_precisa_de_pelo_menos_uma() {
        let (_, errors) = lex(b"1-2");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("pelo menos uma letra"));
    }
}
