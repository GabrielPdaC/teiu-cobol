# teiu-cobol — Especificação léxica e gramatical

Trabalho do Grau A de Compiladores (Unisinos): analisador léxico e gramática da
seção de declarações de COBOL.

> Estado: **esqueleto (F0)**. As seções marcadas com *(F1)*, *(F2)* e *(F4)* são
> preenchidas na fase correspondente.

## 1. Escopo

A linguagem reconhecida é o subconjunto de COBOL que corresponde à parte de
declarações de variáveis da linguagem estilo C do enunciado.

| Enunciado (estilo C)    | Equivalente em teiu-cobol                           |
|-------------------------|-----------------------------------------------------|
| `char`                  | `PIC X` ou `PIC X(n)`                               |
| `int`                   | `PIC 9(n)` ou `PIC S9(n)`                           |
| `float`                 | `PIC 9(n)V9(m)` ou `PIC S9(n)V9(m)`                 |
| `int a, b, c;` (lista)  | item de grupo: `01` com itens subordinados          |
| termina em `;`          | termina em `.`                                      |
| sem inicialização       | sem cláusula `VALUE`                                |
| espaços ignorados       | espaços, tabulações, quebras de linha e comentários `*>` ignorados |

Exemplo de programa válido:

```cobol
DATA DIVISION.
WORKING-STORAGE SECTION.
77 CONTADOR       PIC S9(4).
01 CLIENTE.
   05 NOME        PIC X(30).
   05 SALDO       PIC S9(7)V99.
```

Fora do escopo desta etapa (reconhecidos e rejeitados com erro): `VALUE`,
`USAGE`, `OCCURS`, `REDEFINES`, `FILLER` e os níveis 66 e 88.

## 2. Norma de referência *(F1)*

Revisão da ISO/IEC 1989 adotada e as regras dela que afetam os tokens: tamanho
máximo de nome, caracteres permitidos em nomes e lista de palavras reservadas.

## 3. Alfabeto e formato do fonte *(F1)*

- Formato livre: não há regras de coluna.
- O fonte é lido como bytes; o alfabeto da linguagem é ASCII.
- Maiúsculas e minúsculas são equivalentes em palavras reservadas e nomes.

## 4. Tokens *(F1 rascunho, F2 fechado)*

| Token | Expressão regular | Exemplos | Observações |
|-------|-------------------|----------|-------------|
|       |                   |          |             |

## 5. Gramática das declarações *(F4)*

Gramática em EBNF.

## 6. Regras de hierarquia de níveis *(F4)*

Restrições dependentes de contexto, verificadas com uma pilha no analisador
sintático.

## 7. Tabela de símbolos *(F4)*

Campos registrados para cada item declarado.

## 8. Erros *(F3 e F4)*

Formato das mensagens: `erro <tipo> [linha L, coluna C]: <descrição>`.
Toda mensagem informa a linha.

## 9. Registro de decisões

| # | Decisão | Justificativa |
|---|---------|---------------|
| D1 | Implementação em Rust, linguagem-alvo COBOL | Autorizado pelo professor; o analisador será reaproveitado no trabalho profissional. |
| D2 | Formato livre de fonte, sem regras de coluna | Mantém o foco no reconhecimento de tokens, como no enunciado; o formato fixo pode ser acrescentado depois como uma etapa anterior ao léxico. |
| D3 | Norma ISO genérica, sem extensões de fornecedor | Regras verificáveis em um único documento e defensáveis academicamente. |
| D4 | Subconjunto "espelho estrito" (seção 1) | Corresponde um a um aos três tipos e às regras do enunciado; o escopo reduzido permite dominar o mecanismo por inteiro. |
| D5 | `VALUE` é erro de estrutura, e não erro léxico | `VALUE` é palavra reservada válida de COBOL; o que o enunciado proíbe nesta etapa é a construção (inicialização), não a palavra. |
| D6 | Nenhuma dependência externa; analisador léxico escrito à mão como autômato | Compila em segundos no Colab, cada expressão regular corresponde a um trecho de código explicável e não há código de terceiros a justificar. |
| D7 | Analisador sintático descendente recursivo | A gramática de declarações é LL(1); cada regra vira uma função. |
| D8 | Fonte lido como bytes, e não como texto UTF-8 | O alfabeto é ASCII; um arquivo com acentos ou em Latin-1 gera erro léxico com número de linha, em vez de falhar na leitura. |
| D9 | Códigos de saída: 0 sem erros, 1 com erros no programa, 2 erro de uso ou de leitura | Permite automatizar os testes e distinguir falha do programa analisado de falha da ferramenta. |
