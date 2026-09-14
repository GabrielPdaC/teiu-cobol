*> Programa válido: nomes começando por dígito. Ao contrário de C, COBOL
*> aceita isso (seção 4.3 da especificação) — só exige ao menos uma letra em
*> algum ponto do nome, e não pode começar nem terminar em hífen.
DATA DIVISION.
WORKING-STORAGE SECTION.
01 REGISTRO.
   05 2A-VIA  PIC X(1).
   05 3-CAMPO PIC X(1).
