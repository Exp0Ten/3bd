# Cíle práce

Cílem mé práce bylo vytvořit debugger s grafickým uživatelským prostředím. Program umí ladit kód komilovaných jazyků, a tak pomáhá uživateli při hledání chyb v rámci programování. Za mě nejdůležitější funkce lazení jsou: možnost pozastavení programu, získání aktuální pozice v kódu, zobrazení stavu, ve kterém se nachází, čtení jeho paměti a zajištění komunikace skrze standardní vstup a výstup.
Má snaha byla hlavně vytvořit software, který je příjemný pro používání, dostatečně přizpůsobitelný a v ne menší řadě dále rozvíjitelný.
Pochopitelně jsem také chtěl získat další zkušenosti v oblasti operačního systému Linux, nízkoúrovňového programovaní a programovacího jazyka Rust.

# Způsoby řešení a použité postupy




## Cizí kód

Nakonec bych rád vymezil externí části kódu, které jsem ve svém projektu použil.
Při své práci jsem v souboru "trace.rs" vycházel z tutoriálu "Writing a Linux Debugger", který popisuje vývojový postup debuggeru pro Linux v jazyce *c++*. Stejný postup používám ve funckích **insert_breakpoint**, **remove breakpoint**, kde vyměňuji bajt v paměti lazeného programu[_], při operaci **Operation::Continue**, kde nejdřív udělám krok z aktuální pozice a pak až zapínám *breakpointy* a pokračuji ve spuštění programu[_], a ve funkci **handle**, kde při naražení na *breakpoint* dekrementuji *RIP*[_].
V souboru "dwarf.rs" jsem se na několika místech inspiroval příklady z dokumentace knihovny *gimli*. Specificky ve funcki **load_source**, kde jsem použil příklad uvedený pro čtení *řádkových programů*[_]. Ve funkci **load_dwarf** používám kód pro čtení sekcí *DWARF* pro vytvoření struktury **DwarfSections**. A funkce **create_assembly** je napsána podle vzoru na stránkách dokumentace knihovny *iced-x86*.

# Programovací prostředky

## Programovací jazyk Rust, rustc a rust-analyzer

Pro tento projekt jsem zvolil programovací jazyk Rust (dále jen "Rust") kvůli jeho stabilitě, bezpečnosti a rychlosti. Mám v něm také nejvíce zkušeností a chci se v něm dál rozvíjet.
Pro samotné programování byly nejužitečnějšími nástroji kompiler tohoto jazyka, zvaný *rustc*, a doplněk pro VSC *rust-analyzer*. Dohromady zajišťovaly příjemné vývojové prostředí: doplňování a navrhování funckí a metod a podrobné vysvětlení chyb v kódu.

## Visual Studio Code (VSC)

Na psaní kódu jsem používal textový editor VSC. Pro účely programování mi vyhovuje z několika důvodů: 
- široká nabídka doplńků a jejich jednoduchá správa
- bohaté zvýrazňování v kódu
- možnost kompletního uživatelského přizpůsobení
- rychlé hledání v textu a funkce multi-kursoru

## GNU Binutils - readelf, objdump

Tyto nástroje mi pomáhali se vyznat v informacích pro lazení, v struktuře spustitelných souborů *ELF* a v čtení *strojového kódu*. Díky těmto programům jsem se praktickou metodou naučil strukturu standardu *DWARF*.

## GNU Make

Ve mém projektu se také objevuje několik souborů scriptovacího jazyka *GNU Make* (dále jen "make"). *Make* zjednodušuje celý proces kompilace a používám ho jak pro samotný debugger, tak pro příklady *zdrojových kódů* ve složce "examples".

## Microsoft Copilot

Při práci na projektu jsem také v některých částech použil kód, popřípadě konzultaci s generativní umělou inteligencí Copilot. Rád bych tyto pasáže zde vyjmenoval a upřesnil, co je tvorba má a co jsem použil:

V souboru "data.rs" jsem si nevěděl rady s globálními proměnnými. Copilot mi doporučil použití typu standardní knihovny **Mutex**, který zajišťuje jejich synchronizitu a bezpečnost čtení. Také jsem zjistil jak spravovat statické reference, a to hlavně pro obsah laděného programu.
V souboru "object.rs" jsem si nechal vysvětlit jak fungují pseudoterminály a jak z nich vytvořit standardní komunikaci s laděným kódem.
V souboru "dwarf.rs" mi Copilot vygeneroval makro, které implementuje methody převádějící bajty do různých typů čísel, abych nemusel vypisovat repetetivní kód. Také funkce **align_pointer** pro vytváření assemblerového kódu byla navržená Copilotem.
A nakonec jsem potřeboval vysvětlit proces a implementaci získávání návrátové adresy funkce (anglicky *return address*). Copilot mi nabízel vypočítávání pozice s touto adresou přes odchylku, já jsem však nakonec použil vnitřní funkci knihovny *gimli*, která odvozuje tyto hodnoty přes informace k lazení kódu.

Všechny zmíněné části kódu jsou označené komentářem "//AI".

## Použité knihovny

### nix

Knihovna *nix* je nádstavbou Standardní knihovny pro jazyk C (dále jen libc). Použil jsem ji hlavně pro funcke *ptrace*, *process* a *term*.

### iced

Knihovna *iced* přináší jednoduché prostředí pro tvorbu grafických aplikacích. Ačkoliv je stále ve vývoji, nabízí bohatou škálu přehledných funkcí pro tvorbu grafiky a interakce s uživatelem.

### gimli

Knihovna *gimli* spravuje informace pro lazení, zejména ty pod standardem *DWARF*. Používám ji pro zobrazování *zdrojového kódu*,  a sledování lokálních proměnných.

### Další

Mimo hlavní knihovny jsem dále použil: 
- *iced-x86* jako disassembler strojového kódu pro lazení i na nižší úrovni
- *toml* pro nahrávání a parsování konfiguračních souborů
- *rfd* na vybírání programu skrze průzkumníka souborů a také komunikaci s uživatelem přes tzv. dialogová okna
- *rust-embed* pro kompilaci všech externích souborů (tzv. *assets*) do výsledného spustitelného kódu (vytvoření tzv. *single-file binary executable*)
- *std* jako standardní knihovnu *Rustu* pro obecné funkce, struktury a další
# Zhodnocení dosažených výsledků

# Instalace

# Ovládání

# Seznam použitých informačních zdrojů
