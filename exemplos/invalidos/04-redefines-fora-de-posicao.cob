*> Programa inválido: REDEFINES não vem logo depois do alvo (nem de outra
*> redefinição do mesmo alvo) — NOME está no meio (decisão D24, seção 6.1).
DATA DIVISION.
WORKING-STORAGE SECTION.
01 PESSOA.
   05 CPF-CNPJ PIC X(14).
   05 NOME     PIC X(30).
   05 CPF      REDEFINES CPF-CNPJ PIC 9(11).
