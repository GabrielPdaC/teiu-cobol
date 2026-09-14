# teiu-cobol — Especificação léxica e gramatical

Trabalho do Grau A de Compiladores (Unisinos): analisador léxico e gramática da
seção de declarações de COBOL.

> Estado: **léxico e sintático executáveis, com tabela de símbolos e suíte de
> testes (F2+F4+F5)**. Faltam o notebook do Colab e o relatório (F3, F6).

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
| (sem equivalente direto) | `REDEFINES`: um item ocupando o espaço de outro já declarado (decisão D24) |

Exemplo de programa válido:

```cobol
DATA DIVISION.
WORKING-STORAGE SECTION.
77 CONTADOR       PIC S9(4).
01 CLIENTE.
   05 NOME        PIC X(30).
   05 SALDO       PIC S9(7)V99.
   05 CPF-CNPJ    PIC X(14).
   05 CPF         REDEFINES CPF-CNPJ PIC 9(11).
   05 CNPJ        REDEFINES CPF-CNPJ PIC 9(14).
```

`REDEFINES` foi acrescentado ao escopo por pedido do usuário, para um caso de
uso real do trabalho (documento CPF ou CNPJ no mesmo campo) — é a única
cláusula desta etapa sem equivalente na linguagem estilo C do enunciado, por
isso a seção 6 detalha a regra à parte.

Fora do escopo desta etapa (reconhecidos e rejeitados com erro): `VALUE`,
`USAGE`, `OCCURS`, `FILLER` e os níveis 66 e 88.

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
| `Redefines` | `REDEFINES` | 4 | | | reservada (decisão D24) |
| `UnsupportedReserved` | `VALUE\|USAGE\|OCCURS\|FILLER` | 4 | `VALUE` | | reconhecida para gerar erro de estrutura (D5, D18) |
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
| `CPF REDEFINES CPF-CNPJ PIC 9(11).` | `Name(CPF)` `Redefines` `Name(CPF-CNPJ)` `Pic` `PicString(9(11))` `Period` |

Verificação: as regex foram conferidas contra as colunas *Aceita* e *Rejeita*
(44 casos), e a semântica de casamento mais longo, prioridade e modos foi simulada
sobre 21 entradas, incluindo as desta tabela. Nenhuma falhou. Na F2 esses casos
viram testes unitários em Rust.

### 4.6 Erros léxicos

Toda mensagem informa linha e coluna e é escrita em português (D19).
Implementado em `src/lexer.rs` e coberto por testes automatizados
(`cargo test`).

| Token | Situação | Exemplo | Mensagem prevista |
|-------|----------|---------|-------------------|
| `InvalidWord` | caractere não permitido | `CONTA@X`, `A.B` | palavra inválida `CONTA@X`: caractere `@` não permitido |
| `InvalidWord` | hífen no início ou no fim | `-CONTA`, `CONTA-` | palavra inválida `-CONTA`: nome não pode começar com hífen |
| `InvalidWord` | palavra sem letra que não é número | `1-2` | palavra inválida `1-2`: nome precisa de pelo menos uma letra |
| `Name` | nome longo demais | nome com 32 caracteres | nome excede o limite de 31 caracteres |
| `InvalidPicString` | cadeia PIC fora do subconjunto | `Q(3)`, `X9` | cadeia PIC inválida `Q(3)` |

O diagnóstico de `InvalidWord` testa as situações na ordem da tabela e informa a
primeira que se aplica.

## 5. Gramática das declarações

Implementada em `src/parser.rs` por um analisador descendente recursivo
(decisão D7): cada regra abaixo é uma função, que chama a próxima e verifica o
token atual antes de consumi-lo.

```ebnf
programa            = cabecalho , { entrada } ;
cabecalho           = "DATA" , "DIVISION" , "." , "WORKING-STORAGE" , "SECTION" , "." ;
entrada             = NIVEL , NOME , [ clausula_redefines ] , [ clausula_pic ] , "." ;
clausula_redefines  = "REDEFINES" , NOME ;
clausula_pic        = ( "PIC" | "PICTURE" ) , [ "IS" ] , CADEIA_PIC ;
```

`NIVEL`, `NOME` e `CADEIA_PIC` são os tokens `Number`, `Name` e `PicString` da
seção 4. Não há um `FIM` explícito na gramática: `programa` simplesmente
termina quando os tokens acabam — o analisador lê `entrada` enquanto houver
token, e para no fim do arquivo.

A gramática é LL(1): basta olhar o próximo token para saber qual regra usar.
Em `entrada`, por exemplo, o primeiro token (`NIVEL`) já é suficiente, e dentro
dela a presença ou não de `PIC`/`PICTURE` decide se `clausula_pic` é lida.

## 6. Regras de hierarquia de níveis

A gramática da seção 5 não diz **quem é filho de quem** — dois `entrada`
seguidos são sintaticamente iguais, sejam eles irmãos (`01` e `01`) ou pai e
filho (`01` e `05`). Essa relação depende do valor dos níveis já vistos antes,
o que é contexto, e não estrutura da frase — por isso é verificada à parte,
depois que todas as entradas já foram lidas (decisão D23), com uma pilha:

- Uma entrada nível `L` fecha (retira da pilha) todo item aberto de nível `≥ L`.
- O pai da entrada é o que sobrar no topo da pilha depois disso; se a pilha
  ficar vazia, a entrada não tem pai.
- Um item **sem** `PIC` é empilhado (pode ganhar filhos); um item **com** `PIC`
  não é (é elementar, não tem filhos).
- Um item de nível `77` esvazia a pilha inteira antes (não pode ter pai) e
  nunca é empilhado (não pode ter filhos) — é sempre uma entrada isolada.

Com isso, as regras exigidas ficam assim:

| Regra | Verificação | Mensagem quando falha |
|-------|-------------|------------------------|
| Nível 02-49 precisa de um grupo aberto de nível menor | pilha vazia depois de fechar os níveis `≥ L` | "nível NN de 'nome' precisa estar subordinado a um item de nível menor" |
| Nível 77 não pode ter pai | garantido por esvaziar a pilha antes | (nunca falha: 77 nunca tem pai) |
| Nível 77 não pode ter filhos | garantido por nunca empilhar um 77 | (a entrada seguinte simplesmente não o acha como pai) |
| Item de grupo (sem PIC) precisa de ao menos um filho | ao final, todo item sem PIC que nunca virou pai de ninguém | "item 'nome' incompleto: faltou a cláusula PIC, e o item também não tem nenhum subordinado" |
| Item de nível 77 sempre precisa de PIC | caso particular da regra anterior | "item 'nome' incompleto: o nível 77 sempre exige a cláusula PIC" |

O nível `01` é o único caso em que a pilha vazia é esperada (ele sempre começa
um registro novo), então a regra da primeira linha não se aplica a ele.

### 6.1 A cláusula `REDEFINES`

`REDEFINES` (decisão D24) não muda a hierarquia acima — o item que redefine
continua um irmão comum, no lugar de virar filho de ninguém. O que ela muda é
o **espaço ocupado**: em vez de reservar um espaço novo, o item passa a
ocupar o mesmo espaço de outro já declarado. É assim que se modela um campo
de tamanho variável, como um documento que pode ser CPF (11 dígitos) ou CNPJ
(14 dígitos):

```cobol
05 CPF-CNPJ PIC X(14).
05 CPF       REDEFINES CPF-CNPJ PIC 9(11).
05 CNPJ      REDEFINES CPF-CNPJ PIC 9(14).
```

A verificação, feita depois da hierarquia (`check_redefines` em
`src/parser.rs`), é:

- O alvo (`CPF-CNPJ`) precisa existir e ser elementar (ter PIC) — nesta etapa,
  `REDEFINES` só é aceito entre itens elementares, e não entre grupos.
- O item que redefine também precisa ser elementar.
- A entrada de `REDEFINES` precisa vir **logo depois** do alvo — ou logo
  depois de **outra** entrada que já redefine o mesmo alvo. É essa segunda
  parte da regra que permite `CPF` **e** `CNPJ` redefinirem os dois o mesmo
  `CPF-CNPJ`, um depois do outro: `CNPJ` escreve `REDEFINES CPF-CNPJ`, e não
  `REDEFINES CPF`, mesmo vindo logo depois de `CPF` no arquivo.
- Os dois precisam estar no mesmo nível.

Como os dois ocupam o mesmo espaço, o tamanho do grupo (`PESSOA`, no exemplo)
conta esse espaço **uma vez só**, usando o maior tamanho entre o alvo e todas
as suas redefinições (decisão D25): no exemplo, 14 bytes (de `CPF-CNPJ` ou de
`CNPJ`, que empatam), e não 14+11+14.

## 7. Tabela de símbolos

Uma entrada por item declarado, na ordem em que aparece no programa
(`src/symbols.rs`):

| Campo | Descrição |
|-------|-----------|
| `name` | nome do item, como foi escrito |
| `level` | nível COBOL (01-49 ou 77) |
| `kind` | `Group`, `Char { len }`, `Int { signed, len }` ou `Float { signed, int_len, frac_len }` — a leitura da cadeia PIC (seção 4.3), já traduzida para os três tipos do enunciado |
| `pic` | a cadeia PIC como foi escrita, ou nenhuma nos itens de grupo |
| `size_bytes` | tamanho do item; num item elementar, a soma das posições do PIC (decisão D21); num grupo, a soma dos filhos diretos, calculada depois que a hierarquia da seção 6 é resolvida |
| `parent` | nome do pai na hierarquia, ou nenhum no topo |
| `redefines` | nome do item que este redefine (seção 6.1), ou nenhum |
| `line` | linha onde o nível foi declarado |

## 8. Erros

Formato das mensagens: `erro de <tipo> [linha L, coluna C]: <descrição>`, com
`<tipo>` sendo `léxico` (seção 4.6) ou `estrutura` (seções 5 e 6). Toda
mensagem informa a linha; `main.rs` lista as duas listas juntas, ordenadas por
linha, porque é assim que um usuário lê o arquivo de cima para baixo.

O analisador sintático se recupera de um erro em modo pânico (decisão D22):
ao encontrar um problema numa `entrada`, ele avança até o próximo `.` e
continua a partir da entrada seguinte, em vez de parar no primeiro erro — é
assim que o exemplo `01 A B PIC X.` gera só uma mensagem, e as declarações
depois dele continuam sendo verificadas normalmente.

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
| D8 | Fonte lido como bytes, e não como texto UTF-8 | O alfabeto é ASCII; um arquivo com acentos ou em Latin-1 gera erro léxico com número de linha, em vez de falhar na leitura. Confirmado na F2: `#[logos(utf8 = false)]` faz o `logos` operar sobre `&[u8]`. |
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
| D20 | `src/token.rs` só define os dois `enum` do `logos` (`NormalToken`, `PicToken`); `src/lexer.rs` conduz a troca de modo com `Lexer::morph`, localiza linha/coluna e produz o `Token` público e as mensagens de erro | Separa o que é gerado pela macro do `logos` (tokens e regras) do que é escrito à mão (posição, diagnóstico, troca de modo), o que facilita mostrar cada trecho na apresentação. |
| D21 | Tamanho de um item elementar é a soma das posições do PIC (`X`→1 byte, `9`→1 byte, sinal não conta byte à parte) | O enunciado não pede um formato de armazenamento específico; a leitura "de exibição" (um byte por posição) é a mais simples de explicar e testar. Formatos binários/empacotados ficam fora do escopo (seção 10, L6). |
| D22 | Recuperação de erro em modo pânico: ao falhar numa `entrada`, o sintático avança até o próximo `.` e continua dali | É a técnica mais simples de recuperação de erro sintático e já resolve o requisito do enunciado de relatar mais de um erro por execução; outras técnicas (conjuntos de sincronização, correção automática) são desproporcionais ao tamanho da gramática. |
| D23 | A hierarquia de níveis (seção 6) é verificada num segundo passo, depois que `src/parser.rs` já montou todos os símbolos, e não durante a leitura de cada `entrada` | O tamanho de um grupo depende dos filhos, que só se sabe todos depois de ler o programa inteiro; fazer isso num segundo passo, com uma pilha, evita calcular o tamanho de um grupo antes de conhecer todos os seus membros. |
| D24 | `REDEFINES` entra no escopo (seção 6.1), restrito a itens elementares (com PIC) de ambos os lados; a entrada precisa vir logo depois do alvo ou de outra redefinição do mesmo alvo | Pedido do usuário, para um caso de uso real (documento CPF/CNPJ no mesmo campo). É a única cláusula fora da correspondência com a linguagem estilo C do enunciado. A restrição a elementares evita a complexidade de um grupo redefinir outro grupo (recalcular o tamanho de uma subárvore inteira), que fica como trabalho futuro (L9). A regra de adjacência (D22, no sentido de manter a leitura simples) é a mesma que a maioria dos compiladores COBOL exige. |
| D25 | O tamanho de um grupo conta um item redefinido **uma vez só**, como o maior tamanho entre ele e todas as suas redefinições | `REDEFINES` significa que os itens dividem o mesmo espaço de memória, e não que cada um tem o seu; somar os dois contaria espaço em dobro. |

## 10. Limitações conhecidas

| # | Limitação | Consequência |
|---|-----------|--------------|
| L1 | Lista parcial de palavras reservadas (D18) | `01 MOVE PIC X.` é aceito, embora `MOVE` seja reservada em COBOL. |
| L2 | Literais alfanuméricos (`"ABC"`) não fazem parte do subconjunto | `VALUE "A"` gera erro léxico nas aspas, além do erro de estrutura do `VALUE`. |
| L3 | Coluna contada em bytes | Um caractere acentuado em comentário ocupa mais de uma coluna. Não afeta a linha. |
| L4 | PIC editados (`Z`, `*`, `,`, `.` inserido) não são aceitos | `PIC 9(5).99` e `PIC ZZ9` geram erro de cadeia PIC. |
| L5 | O comentário `*>` precisa começar um token | Em `PIC X.*> texto`, sem espaço antes do `*>`, a sequência `X.*>` vira `InvalidPicString`. |
| L6 | Tamanho em bytes não representa formatos binários/empacotados (decisão D21) | `PIC S9(4) USAGE COMP` teria o mesmo tamanho calculado que sem `USAGE`, embora ocupe menos bytes de verdade — mas `USAGE` está fora do escopo (seção 1), então isso não afeta os programas aceitos nesta etapa. |
| L7 | Nomes repetidos no mesmo programa não são detectados como erro | `01 A PIC X.` seguido de outro `01 A PIC X.` é aceito; o segundo sobrescreve o primeiro na verificação de hierarquia (seção 6), já que ela indexa por nome. |
| L8 | Um item elementar (com PIC) que recebe um item de nível menor logo depois não é sinalizado como erro específico | Como um item com PIC nunca é empilhado (seção 6), o nível seguinte procura o pai mais acima e pode encontrar um grupo mais distante, em vez de acusar "item elementar não pode ter subordinados". |
| L9 | `REDEFINES` só é aceito entre itens elementares (D24) | `05 GRUPO-B REDEFINES GRUPO-A.`, com os dois sendo grupos, não é aceito nesta etapa. |
