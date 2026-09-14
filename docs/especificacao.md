# teiu-cobol — Especificação léxica e gramatical

Trabalho do Grau A de Compiladores (Unisinos): analisador léxico e gramática da
seção de declarações de COBOL.

> Estado: **rascunho dos tokens (F1)**. As seções marcadas com *(F2)* e *(F4)*
> são fechadas na fase correspondente.

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

## 2. Norma de referência

Revisão adotada: **ISO/IEC 1989:2014** (decisão D10).

O texto da norma é pago e não estava disponível. As regras que afetam os tokens
foram confirmadas em fontes secundárias públicas:

| Regra | Valor adotado | Fonte |
|-------|---------------|-------|
| Tamanho máximo de nome | 31 caracteres | Perfil de conformidade *COBOL 2014* do GnuCOBOL (`word-length: 31`). Para comparação, o perfil *COBOL 85* usa 30. |
| Caracteres de um nome | letras, dígitos e hífen | Documentação do IBM Enterprise COBOL |
| Posição do hífen | nem no início, nem no fim | Documentação do IBM Enterprise COBOL |
| Letra obrigatória | pelo menos uma letra em nomes de dados | Documentação do IBM Enterprise COBOL |
| Maiúsculas e minúsculas | equivalentes | Documentação do IBM Enterprise COBOL |
| Sublinhado (`_`) | **não aceito** (decisão D11) | A IBM aceita, mas não foi possível confirmar na ISO. |

Fontes:
[perfil COBOL 2014 do GnuCOBOL](https://github.com/OCamlPro/gnucobol/blob/gcos4gnucobol-3.x/config/cobol2014.conf),
[perfil COBOL 85 do GnuCOBOL](https://github.com/OCamlPro/gnucobol/blob/gcos4gnucobol-3.x/config/cobol85.conf),
[IBM — COBOL words with single-byte characters](https://www.ibm.com/docs/en/cobol-zos/6.3.0?topic=literals-cobol-words-single-byte-characters),
[ISO/IEC 1989:2014](https://www.iso.org/standard/51416.html).

## 3. Alfabeto e formato do fonte

- **Formato livre:** não há regras de coluna.
- **Alfabeto:** a linguagem usa apenas ASCII. Um caractere fora do ASCII só é
  aceito dentro de comentários; em qualquer outro lugar gera erro léxico (D8).
- **Maiúsculas e minúsculas** são equivalentes em palavras reservadas, nomes e
  cadeias PIC. O lexema é guardado como foi escrito.
- **Espaços em branco:** espaço, tabulação, `CR` e `LF` separam tokens e são ignorados.
- **Comentário:** `*>` no início de um token inicia um comentário que vai até o
  fim da linha.
- **Posição:** a linha conta as quebras `LF`, começando em 1. A coluna é a
  posição em bytes dentro da linha, começando em 1.

## 4. Tokens

Os nomes dos tokens são os mesmos das variantes do `enum` no código (D19).

### 4.1 Como o analisador léxico escolhe o token

O analisador é gerado pela biblioteca `logos` (D6), que segue a mesma semântica
do Flex:

1. **Casamento mais longo:** entre todas as regras, vence a que casa com o maior
   trecho da entrada a partir da posição atual.
2. **Prioridade:** se duas regras casam com o mesmo tamanho, vence a de maior
   prioridade. É o equivalente, no Flex, a declarar a regra antes.

Ordem de prioridade, da maior para a menor:

| Prioridade | Regras |
|------------|--------|
| 5 | espaços e comentários (ignorados) |
| 4 | palavras reservadas |
| 3 | `Number`, `PicString` |
| 2 | `Name`, `Period` |
| 1 | `InvalidWord`, `InvalidPicString` (regras pega-tudo de erro) |

O que importa é a ordem; os valores numéricos usados no `logos` são definidos na F2.

Exemplos de desempate:
- **Reservada contra nome:** `DATA` casa com `Data` e com `Name`, ambos com 4
  caracteres, e a reservada vence pela prioridade.
- **Casamento mais longo:** `PICTURES` casa com `Name` com 8 caracteres e com
  `Pic` só com 7, então é um nome.

**Autômato, e não backtracking.** O `logos`, assim como o Flex, compila as regras
num autômato finito determinístico, que sempre encontra o casamento mais longo.
Com `PIC|PICTURE`, a entrada `PICTURE` casa por inteiro. Um motor com
backtracking, como o `re` do Python, testa as alternativas em ordem e pararia em
`PIC`. Isso apareceu na prática ao simular estas regras em Python.

**Regras pega-tudo de erro** (D12): `InvalidWord` casa com qualquer sequência
sem espaços que não termine em ponto. Ela tem a menor prioridade, então só vence
quando casa com um trecho **mais longo** que qualquer token válido:
- `-CONTA` (6 caracteres) vence `Name`, que não casa a partir de `-`, e gera um
  único erro para a palavra inteira;
- `CONTA-` (6) vence `Name`, que só casaria `CONTA` (5);
- `CLIENTE.` → `Name` e `InvalidWord` casam os mesmos 7 caracteres, a
  prioridade dá a vitória a `Name` e o ponto sobra para `Period`.

**Modos** (D15), o equivalente às *start conditions* do Flex:

```
         Pic                      PicString | InvalidPicString | Period
NORMAL  ─────►  PIC   ────────────────────────────────────────────────►  NORMAL
                 │ ▲
                 └─┘ Is
```

No modo `PIC` só valem as regras da tabela 4.4. É necessário porque `X` e `9(4)`
são cadeias PIC dentro da cláusula, mas não são tokens válidos fora dela.

### 4.2 Subexpressões auxiliares

```
COUNT = \(0*[1-9][0-9]*\)
REPX  = X(COUNT)?
REP9  = 9(COUNT)?
```

### 4.3 Tokens do modo NORMAL

As expressões regulares ignoram maiúsculas e minúsculas.

| Token | Expressão regular | Prior. | Aceita | Rejeita | Observações |
|-------|-------------------|--------|--------|---------|-------------|
| `Data` | `DATA` | 4 | `DATA`, `data` | | reservada |
| `Division` | `DIVISION` | 4 | | | reservada |
| `WorkingStorage` | `WORKING-STORAGE` | 4 | | | reservada |
| `Section` | `SECTION` | 4 | | | reservada |
| `Pic` | `PIC\|PICTURE` | 4 | `PIC`, `PICTURE` | `PICTURES` | reservada; entra no modo PIC |
| `Is` | `IS` | 4 | | | reservada |
| `UnsupportedReserved` | `VALUE\|USAGE\|OCCURS\|REDEFINES\|FILLER` | 4 | `VALUE` | | reconhecida para gerar erro de estrutura (D5, D18) |
| `Number` | `[0-9]+(\.[0-9]+)?` | 3 | `01`, `77`, `10`, `3.5` | `3.`, `.5`, `1-2`, `3,5` | a faixa de nível é verificada pelo sintático (D13, D14) |
| `Name` | `([0-9]+-+)*[0-9]*[A-Z]([A-Z0-9-]*[A-Z0-9])?` | 2 | `CONTADOR`, `WS-CLIENTE`, `2A-VIA`, `1-A`, `A--B` | `-CONTA`, `CONTA-`, `123`, `1-2`, `CONTA_DOR`, `CONTA@X` | no máximo 31 caracteres (D17) |
| `Period` | `\.` | 2 | `.` | | |
| `InvalidWord` | `[^ \t\r\n]*[^ \t\r\n.]` | 1 | | | erro léxico (4.6) |
| *(ignorado)* | `[ \t\r\n]+` | 5 | | | espaços |
| *(ignorado)* | `\*>[^\n]*` | 5 | | | comentário |

O fim da entrada é representado pelo token `Eof`, gerado pelo analisador ao
chegar ao final do arquivo.

**Como ler a regex de `Name`:**
- `([0-9]+-+)*[0-9]*` permite começar com dígitos, inclusive separados por hífen,
  como em `2A-VIA` e `1-A`, mas nunca começar com hífen;
- `[A-Z]` garante pelo menos uma letra;
- `([A-Z0-9-]*[A-Z0-9])?` permite hífens no meio, mas exige que o nome termine em
  letra ou dígito.

Em COBOL, ao contrário de C, `2CONTADOR` **é um nome válido**. Essa regra também
resolve os três erros frequentes do enunciado:
- **Dígito inicial:** é permitido, mas a letra continua obrigatória.
- **Ponto sem escape:** `Number` usa `\.`.
- **Reservada absorvida pelo identificador:** resolvido pela prioridade.

### 4.4 Tokens do modo PIC

| Token | Expressão regular | Prior. | Aceita | Rejeita | Observações |
|-------|-------------------|--------|--------|---------|-------------|
| `Is` | `IS` | 4 | | | permanece no modo PIC |
| `PicString` | `(REPX)+ \| S?((REP9)+(V(REP9)*)? \| V(REP9)+)` | 3 | `X`, `XXX`, `X(30)`, `9(05)`, `S9(4)`, `S9(7)V99`, `V99`, `99V` | `X(0)`, `S`, `V`, `SV`, `X9`, `Q(3)`, `9(5).99`, `X(`, `SX` | volta ao modo NORMAL (D16) |
| `Period` | `\.` | 2 | | | volta ao modo NORMAL; a falta da cadeia é erro de estrutura |
| `InvalidPicString` | `[^ \t\r\n]*[^ \t\r\n.]` | 1 | | | erro léxico (4.6); volta ao modo NORMAL |
| *(ignorado)* | `[ \t\r\n]+` e `\*>[^\n]*` | 5 | | | |

**Como ler a regex de `PicString`:**
- a primeira alternativa é o alfanumérico (`char`);
- a segunda é o numérico, com sinal `S` opcional e o `V`, que marca a vírgula
  decimal implícita (`int` sem `V`, `float` com `V`);
- as alternativas internas exigem pelo menos um `9`, então `S`, `V` e `SV`
  sozinhos são rejeitados.

### 4.5 Exemplos de reconhecimento

| Entrada | Tokens |
|---------|--------|
| `01 NOME PIC X(30).` | `Number(01)` `Name(NOME)` `Pic(PIC)` `PicString(X(30))` `Period` |
| `PIC IS S9(7)V99.` | `Pic` `Is` `PicString(S9(7)V99)` `Period` |
| `data division.` | `Data(data)` `Division(division)` `Period` |
| `2A-VIA` | `Name(2A-VIA)` |
| `CONTA@X.` | `InvalidWord(CONTA@X)` `Period` |
| `A.B.` | `InvalidWord(A.B)` `Period` |
| `X(30)` fora do modo PIC | `InvalidWord(X(30))` |
| `VALUE 3.5.` | `UnsupportedReserved(VALUE)` `Number(3.5)` `Period` |
| `PICTURE 9(5).99.` | `Pic` `InvalidPicString(9(5).99)` `Period` |
| `PIC X9 VALUE` | `Pic` `InvalidPicString(X9)` `UnsupportedReserved(VALUE)` |
| `PIC .` | `Pic` `Period` |

Verificação: as regex foram conferidas contra as colunas *Aceita* e *Rejeita*
(44 casos), e a semântica de casamento mais longo, prioridade e modos foi simulada
sobre 21 entradas, incluindo as desta tabela. Nenhuma falhou. Na F2 esses casos
viram testes unitários em Rust.

### 4.6 Erros léxicos *(F2)*

Toda mensagem informa linha e coluna e é escrita em português (D19).

| Token | Situação | Exemplo | Mensagem prevista |
|-------|----------|---------|-------------------|
| `InvalidWord` | caractere não permitido | `CONTA@X`, `A.B` | palavra inválida `CONTA@X`: caractere `@` não permitido |
| `InvalidWord` | hífen no início ou no fim | `-CONTA`, `CONTA-` | palavra inválida `-CONTA`: nome não pode começar com hífen |
| `InvalidWord` | palavra sem letra que não é número | `1-2` | palavra inválida `1-2`: nome precisa de pelo menos uma letra |
| `Name` | nome longo demais | nome com 32 caracteres | nome excede o limite de 31 caracteres |
| `InvalidPicString` | cadeia PIC fora do subconjunto | `Q(3)`, `X9` | cadeia PIC inválida `Q(3)` |

O diagnóstico de `InvalidWord` testa as situações na ordem da tabela e informa a
primeira que se aplica.

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
| D6 | Dependências: `logos` para gerar o analisador léxico e `clap` para a linha de comando; analisador sintático escrito à mão | O `logos` é o equivalente em Rust do Flex indicado no enunciado: regras com expressões regulares compiladas num autômato, casamento mais longo, prioridade (ordem das regras), *callbacks* (ações) e troca de modo (*start conditions*). Foi preferido ao `lrlex`, que aceita arquivos no formato `.l` mas não tem ações, o que impediria verificar o tamanho de nomes e diagnosticar palavras inválidas no próprio léxico. O `clap` gera a ajuda e a validação dos argumentos. |
| D7 | Analisador sintático descendente recursivo | A gramática de declarações é LL(1); cada regra vira uma função. |
| D8 | Fonte lido como bytes, e não como texto UTF-8 | O alfabeto é ASCII; um arquivo com acentos ou em Latin-1 gera erro léxico com número de linha, em vez de falhar na leitura. *A confirmar na F2: suporte do `logos` a entrada em bytes.* |
| D9 | Códigos de saída: 0 sem erros, 1 com erros no programa, 2 erro de uso ou de leitura | Permite automatizar os testes e distinguir falha do programa analisado de falha da ferramenta. Coincide com o código que o `clap` usa para argumentos inválidos. |
| D10 | Revisão ISO/IEC 1989:2014 | É a revisão mais recente com perfil de conformidade público (GnuCOBOL) para conferir as regras; a de 2023 não tem. |
| D11 | Sublinhado não é aceito em nomes | Não foi possível confirmar na ISO; aceitar só por ser extensão da IBM contrariaria D3. |
| D12 | Regras pega-tudo `InvalidWord` e `InvalidPicString`, com a menor prioridade | Pelo casamento mais longo, uma palavra inválida (`-CONTA`, `CONTA@X`) é consumida inteira e gera **uma** mensagem clara, em vez de um erro por caractere seguido de tokens soltos. Por não terminarem em ponto, não engolem o ponto final. |
| D13 | Token `Number` genérico; a faixa de nível (01–49, 77) é verificada pelo sintático | O léxico não sabe a posição da palavra na entrada: `10` pode ser nível ou vir depois de `VALUE`. Com isso, `88` e `50` viram erros de estrutura com mensagem específica. |
| D14 | `Number` aceita parte decimal, com o ponto escapado (`\.`) | Faz `VALUE 3.5` ser um erro de estrutura (D5) em vez de um erro léxico no `.5`; o escape evita o erro frequente do ponto que casa com qualquer caractere. |
| D15 | Modo PIC no léxico, com `morph` do `logos` | A mesma palavra (`X`, `9`) tem classificação diferente dentro e fora da cláusula PIC; é o recurso de *start conditions* do Flex. |
| D16 | Subconjunto de PIC: sem misturar `X` e `9`, contagem ≥ 1 (zeros à esquerda permitidos), ao menos um `9` nos numéricos | Espelha os três tipos do enunciado; `9(05)` é comum em código real; `X(0)` e `SV` não descrevem dado algum. |
| D17 | Limite de 31 caracteres verificado no *callback* do token `Name`, e não na regex | Uma regex com limite de repetição ficaria ilegível; o *callback* (a ação do Flex) dá uma mensagem específica. |
| D18 | Palavras reservadas restritas às do subconjunto e às cláusulas fora do escopo | A lista completa da ISO tem centenas de palavras e o texto normativo não estava disponível (ver L1). |
| D19 | Código em inglês, comentários e mensagens ao usuário em português; tokens com os mesmos nomes na especificação e no código | O código segue a convenção do ecossistema Rust; o relatório, a apresentação e as mensagens são para a disciplina. Nomes iguais permitem rastrear cada regra da especificação até o código. |

## 10. Limitações conhecidas

| # | Limitação | Consequência |
|---|-----------|--------------|
| L1 | Lista parcial de palavras reservadas (D18) | `01 MOVE PIC X.` é aceito, embora `MOVE` seja reservada em COBOL. |
| L2 | Literais alfanuméricos (`"ABC"`) não fazem parte do subconjunto | `VALUE "A"` gera erro léxico nas aspas, além do erro de estrutura do `VALUE`. |
| L3 | Coluna contada em bytes | Um caractere acentuado em comentário ocupa mais de uma coluna. Não afeta a linha. |
| L4 | PIC editados (`Z`, `*`, `,`, `.` inserido) não são aceitos | `PIC 9(5).99` e `PIC ZZ9` geram erro de cadeia PIC. |
| L5 | O comentário `*>` precisa começar um token | Em `PIC X.*> texto`, sem espaço antes do `*>`, a sequência `X.*>` vira `InvalidPicString`. |
