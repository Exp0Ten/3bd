# Cíle práce

Cílem mé práce bylo vytvořit debugger s grafickým uživatelským rozhraním. Program umí ladit kód kompilovaných jazyků, a tak pomáhá uživateli při hledání chyb v kódu. Nejdůležitější funkce lazení pro mě byly: možnost pozastavení programu, získání aktuální pozice v kódu, zobrazení stavu, ve kterém se nachází, čtení jeho paměti a zajištění komunikace skrze standardní vstup a výstup.
Má snaha byla hlavně vytvořit software, který je příjemný pro používání, dostatečně přizpůsobitelný a v ne menší řadě dále rozvíjitelný.
Pochopitelně jsem také chtěl získat další zkušenosti v oblasti operačního systému Linux, nízkoúrovňového programovaní a programovacího jazyka Rust.

# Způsoby řešení a použité postupy

## Návrh a obecná architektura

## Sledování procesu

## Zpracování informací k lazení

## Uživatelské rozhraní

## Cizí kód

Nakonec bych rád vymezil externí části kódu, které jsem ve svém projektu použil.
Při své práci jsem v souboru "trace.rs" vycházel z tutoriálu "Writing a Linux Debugger", který popisuje psaní debuggeru pro Linux v jazyce *c++*. Stejný postup jako zmíněný zdroj používám ve funckích **insert_breakpoint** a **remove_ breakpoint**, kde vyměňuji bajt v paměti lazeného programu[_], při operaci **Operation::Continue**, kde nejdřív udělám krok z aktuální pozice a pak až vkládám *breakpointy* a pokračuji ve spuštění programu[_], a ve funkci **handle**, ve které rozpoznávám naražení na *breakpoint*[_].
V souboru "dwarf.rs" jsem se na několika místech inspiroval příklady z dokumentace knihovny *gimli*. Specificky ve funcki **load_source**, kde jsem použil příklad uvedený pro čtení *řádkových programů*[_]. Ve funkci **load_dwarf** používám kód pro čtení sekcí *DWARF* pro vytvoření struktury **DwarfSections**[_]. A funkce **create_assembly** je napsána podle vzoru na stránkách dokumentace knihovny *iced-x86*[_].

# Programovací prostředky

## Programovací jazyk Rust, rustc a rust-analyzer

Pro tento projekt jsem zvolil programovací jazyk Rust (dále jen "Rust") kvůli jeho stabilitě, bezpečnosti a rychlosti. Mám v něm také nejvíce zkušeností a chci se v něm dál rozvíjet.
Pro samotné programování byly nejužitečnějšími nástroji kompiler tohoto jazyka, zvaný *rustc*, a doplněk *rust-analyzer* pro *VSC*. Dohromady zajišťovaly příjemné vývojové prostředí: doplňování a navrhování funckí a metod a podrobné vysvětlení chyb v kódu.

## Visual Studio Code (VSC)

Na psaní kódu jsem používal textový editor VSC. Pro účely programování mi vyhovuje z několika důvodů: 
- široká nabídka doplňků a jejich jednoduchá správa
- bohaté zvýrazňování v kódu
- možnost kompletního uživatelského přizpůsobení
- rychlé hledání v textu a funkce multi-kursoru

## GNU Make

Ve mém projektu se také objevuje několik souborů scriptovacího jazyka *GNU Make* (dále jen "make"). *Make* zjednodušuje celý proces kompilace a používám ho jak pro samotný debugger, tak pro příklady *zdrojových kódů* ve složce "examples".

## Microsoft Copilot

Při práci na projektu jsem také v některých částech použil kód, popřípadě konzultaci s generativní umělou inteligencí Copilot. Rád bych tyto pasáže zde vyjmenoval a upřesnil:

V souboru "data.rs" jsem si nevěděl rady s globálními proměnnými. Copilot mi doporučil použití typu standardní knihovny **Mutex**, který zajišťuje jejich synchronizitu a bezpečnost čtení a zápisu. Také jsem zjistil jak spravovat statické reference, a to hlavně pro hrubý obsah laděného programu.
V souboru "object.rs" jsem si nechal vysvětlit jak fungují pseudoterminály a jak z nich vytvořit standardní komunikaci s laděným kódem.
V souboru "dwarf.rs" mi Copilot vygeneroval makro, které implementuje methody převádějící bajty do různých typů čísel, abych nemusel vypisovat repetetivní kód. Také funkce **align_pointer** pro vytváření assemblerového kódu byla navržená Copilotem.
A nakonec jsem potřeboval vysvětlit proces a implementaci získávání návrátové adresy (anglicky *return address*) funkcí. Copilot mi nabízel vypočítávání pozice s touto adresou přes odchylku, já jsem však nakonec použil vnitřní funkci knihovny *gimli*, která odvozuje tyto hodnoty přes informace k lazení.

Všechny zmíněné části kódu jsou označené komentářem "//AI".

## Použité knihovny

### nix

Knihovna *nix* je nádstavbou Standardní knihovny pro jazyk C (známá také jako "libc"). Použil jsem ji hlavně pro moduly *ptrace*, *process* a *term*. *Ptrace* umožňuje sledování procesů, z *process* jsem použil funkci *fork* pro vytvoření nového podprocesu a *term* byl nezbytný pro vytvoření pseudoterminálu na standardní komunikaci. [_]

### iced

Knihovna *iced* přináší jednoduché prostředí pro tvorbu grafických aplikacích. Ačkoliv je stále ve vývoji, nabízí bohatou škálu přehledných funkcí pro tvorbu grafiky a interakce s uživatelem. Taky jsem s ní s měl trochu zkušeností. [_]

### gimli

Knihovna *gimli* spravuje informace pro lazení, zejména ty pod standardem *DWARF*. Jedná se o velmi silný modul na *parsování* těchto dat. Používám ji pro zobrazování *zdrojového kódu*,  a sledování lokálních proměnných. [_]

### Další

Mimo hlavní knihovny jsem dále použil:
- *iced-x86* jako disassembler strojového kódu pro lazení i na nižší úrovni [_]
- *toml* pro nahrávání a parsování konfiguračních souborů [_]
- *rfd* na vybírání programu skrze průzkumníka souborů a také komunikaci s uživatelem přes tzv. dialogová okna [_]
- *rust-embed* pro kompilaci všech externích souborů (tzv. *assets*) do výsledného spustitelného kódu (vytvoření tzv. *single-file binary executable*) [_]
- *std* jako standardní knihovnu *Rustu* pro obecné funkce, struktury a další [_]

Široký seznam knihoven byl pro mě bližší nežli závislost projektu na externích programech.

## Ikony Adwaita

V práci také používám soubor ikon *Adwaita* [_] vytvořený pod projektem *GNOME*. Ve složce "code/src/assets/icons" se s použitými ikonami nachází soubory "LICENSE" a "README.md", které upřesňují licencování této grafiky. Všechny ikony, vyjma souboru "TBD.svg", licencuji pod licencí "GNU Lesser General Public License v3.0". Ta je kompatibilní s tou, která licencuje celý můj projekt ("GNU General Public License v3.0").

# Zhodnocení dosažených výsledků

Ačkoliv jsem nedosáhl všech mých původních představ, věřím, že jsem zadání splnil a osobně jsem s prací velmi spokojený. Myslím si, že jsem dokázal vytvořit software, který je funkčí, přizpůsobitelný a rozvíjitelný. Rád bych však vyjmenoval některé nedostatky, které v práci cítím.
Chybí mi podrobnější barevné zvýrazňování v aplikaci, hlavně v panelech **PaneCode** a **PaneStack**. Stejně tak jsem zjistil při zkoušení dalších vestavěných barevných schémat z knihovny *iced*, že v některých z nich je text špatně čitelný.
Dále mi nevyhovuje funkce panelu **PaneAssembly**, jelikož se nedá prohlížet *assemblerový kód* daleko od aktuální pozice v kódu.
Určitě bych panelu **PaneStack** přidal možnost zobrazení hodnot proměnných v různých formátech a také funkci připnutí některých rádků pro větší přehlednost.
A nakonec mi chybí klávesové zkratky ovládající aplikaci a vybírání souboru přes argumenty příkazu při spouštění aplikace.
Program plánuji nadále vylepšovat a rozšiřovat, hlavně za účelem odstranění zmíněných nedostatků.

# Instalace

Ve složce *build* se nachází soubor *Makefile*, který obsahuje script pro program *make*. Stačí tedy v tomto adresáři zadat příkaz "make" (nebo "make all") a celý program se zkompiluje a vytvoří se zde spustitelný soubor "tbd". Ten se dá poté přímo spustit pomocí "./tbd". (Pro instalaci na použití kdekoliv v systému můžeme přesunout soubor do "/bin" nebo "/usr/bin" nebo přidat cestu k souboru do systémové proměnné "$PATH".)
Program se dá spustit i mimo prostředí terminálu.

## Nároky a kompatibilita

Program podporuje pouze operační systém Linux, měl by však fungovat na většině distribucí. Měl by fungovat v desktopových prostředích *x11* i *wayland*.
Výsledný soubor má 21MB, a z mého testování program nikdy nevyžadoval více než 120MB paměti. Celková zátěž paměti se však zvyšuje dvojnásobně s velikostí lazeného kódu, protože jeho obsah je načten pro zpracování a pak ještě spuštěn. Debugger vyžaduje největší výkon při zvolení souboru kvůli zpracování dat. Při velkém množství informací k lazení může tento proces trvat několik vteřin, například u programů psaných v *Rustu*. Ke zpomalení grafiky také může dojít, pokud debugger zobrazuje velký text *zdrojového kódu*. 
Program využívá tzv. *procfs*. Jedná se správu běžících procesů skrze souborovým systém. Pro správný chod programu je nutné, aby prostředí toto rozhraní podporovalo. Toto můžete ověřit promocí příkazu "test -d /proc && echo true" (program vypíše "true", pokud najde složku tohoto rozhraní).
Všechen kód byl vyvíjen a důkladně testován v distribuci Debian 12 Bookworm a v prostředí *wayland*, specificky v KDE Plasma. Úplná kompatibility byla také zajištěna pro Debian 13 Trixie.

## Externí závislosti

Na kompilaci kódu je potřeba sada nástroju pro Rust s názvem *rustup*. [_]
Dále pro dialogová okna je nutný program *zenity*. Tato závislost není povinná, ale je silně doporučená. [_]
Projekt používá *GNU Make* na jednodušší kompilaci a jeho instalace je pro zajištění správného fungování potřebná. [_]
Pro kompilaci příkladů kódu ve složce "examples" jsou vyžadovány kompilery *GNU C Compiler*[_] a *GNU C++ Compiler*[_], také známé jako *gcc* a *g++*.
A v neposlední řadě je nutná *Knihovna GNU C*, také známa jako *glibc*. Na debianu se specificky jedná o balík *libc6*. [_]
Popis instalace těchto závislostí pro distribuce Debian a Ubuntu se nachází v souboru "DEPENDENCIES.md".

# Ovládání

## Popis uživatelského rozhraní

### Okno, toolbar, panely

### Panely podrobně

### Dialogová okna

### Nastavení

## Typický postup při používání

### Po spuštění

### Vybrání programu a příprava na lazení

### Začátek a průběh lazení

### Ukončení programu

## Další informace

# Seznam použitých informačních zdrojů
