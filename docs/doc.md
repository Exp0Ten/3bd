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

V práci také používám soubor ikon *Adwaita* [_] vytvořený pod projektem *GNOME*. Ve složce "code/assets/icons" se s použitými ikonami nachází soubory "LICENSE" a "README.md", které upřesňují licencování této grafiky. Všechny ikony, vyjma souboru "TBD.svg", licencuji pod licencí "GNU Lesser General Public License v3.0". Ta je kompatibilní s tou, která licencuje celý můj projekt ("GNU General Public License v3.0").

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

Celé uživatelské rozhraní se skládá ze tří částí. Tou nejdůležitější je hlavní okno, přes ktére se program ovládá, a kde se zobrazují všechny informace při lazení kódu. Další jsou menší dialogové okna, která sdělují uživateli zásadní zprávy v průběhu používání. Třetí z nich je nastavení skrze konfigurační soubory. 

### Okno, toolbar, panely

Okno se dělí na *toolbar* nahoře, *statusbar* dole, a *mainframe* uprostřed.
*Toolbar* obsahuje tlačítko na vybrání souboru vlevo a tlačítka na otevření či schování postranních panelů vpravo.
*Statusbar* obsahuje obecné informace jako název vybraného souboru, identifikační číslo procesu (tzv. "PID"), stav procesu a popřípadě na jakém řádku *zdrojového kódu* je proces zastaven.
*Mainframe* obsahuje panely, které zobrazují veškeré informace o procesu a umožňují lazení kódu. Tyto panely můžeme přesouvat, měnit jejich velikost, a tak si přizbůsobit celé grafické prostředí.
Každý panel má svůj titulek se jménem a pod ním svůj obsah. V horní části obsahu se soustřeďují interaktivní struktur, například tlačítka a výběry seznamů. Většina panelů má své vnitřní hodnoty, a proto můžeme používat i více panelů stejného typu zároveň.

### Panely podrobně

Panelů je celkem 8 a každý plní jinou funkci.

Panel "Control" obsahuje tlačítka na ovládání lazeného procesu. Zleva doprava jdou následovně: "Spustit/Zastavit", "Pokračovat/Pozastavit", "Krok", "Krok v *zdrojovém kódu*", "Ukončit", "Poslat Signál". Úplně vpravo poté můžeme vybrat signál, který chceme procesu poslat. "Krok" znamená spustit další instrukci, zatímco "Krok v *zdrojovém kódu*" spustí program a zastaví se jakmile narazí na pozici, která je spojená s pozicí v *zdrojovém kódu*.
Panel "Memory" zobrazuje obsah paměti programu. V poli můžeme zadat adresu v šestnáctkovém nebo desítkovém zápisu. Tlačítko úplně vlevo mění počet zobrazovaných bajtů na řádek. Zbylá tlačítka specifikují formát, ve kterém jsou jednotlivé bajty zobrazovány. V těle panelu se pak zobrazují řádky s adresami a bajty. Adresa vždy ukazuje pozici prvního bajtu v řádku. Při použití kolečka na myši se můžeme pohybovat nahoru a dolu.
Panel "Code" zobrazuje zdrojový kód lazeného programu. Nahoře můžeme vybrat tzv. kompilační složku a soubor. Tlačítko vpravu udává, jestli chceme sledovat aktuální pozici v kódu. Pozice je zobrazena přes zvýrazněné číslo řádku, na kterém právě jsme. Pokud řádek má k sobě přiřazenou adresu, můžeme na něj dát *breakpoint*. Tlačítko na breakpoint se nachází vlevo od čísla řádku.
Panel "Registers" zobrazuje hodnotu registrů procesoru. Tlačítka nahoře číselnou soustavu, ve které jsou hodnoty zobrazovány.
Panel "ELF Info" vypisuje informace, které nalezneme v hlavičce *ELF* souborů. Jedná se například o cílený operační systém a architekturu, vstupní adresu kódu a další.
Panel "Terminal" funguje jako interní terminál pro stadardní komunikaci s lazeným programem. Nahoře je stadardní výstup, dole je pole pro standardní vstup. Aktuální pozice kursoru je zobrazována znakem '_'. Pro poslání standardního vstup je nutné zmáčknout klávesu "Enter".
Panel "Assembly" vypisuje okolní assemblerový kód k aktuální pozici. Ta je značena zvýrazněnou adresou vlevo. Instrukce jsou zobrazovány ve formátu assembleru *nasm*. Jsou vypsány i bajty každé instrukce. Vedle adres jsou také tlačítka na *breakpointy*.
Panel "CallStack" vypisuje tzv. *callstack*, neboli seznam funkcí podle toho, jak byly postupně volány. U každé funkce pak vypisuje deklarované proměnné a jejich typy a hodnoty. Řádek značící funkce je zvýrazněný. "0" znamená prvně zavolaná funkce, zpravidla "main". Funkce, struktury a seznamy vytváří odsazení v zobrazování. Proto vedle každého z nich je pro větší přehlednost tlačítko na schování či rozbalení jeho obsahu. Při aktualizaci je vždy obsah aktuální funkce rozbalen zatímco všech ostatní schován.

### Dialogová okna

Dialogová okna sdělují důležité informace uživateli. Jedná se o chyby při lazení kódu, informace o průběhu spuštěného procesu a varování při nestandardních situacích.
Zprávy o chybách vždy zobrazí příčinu chyby a případně další informace o selhání některé z funkcí. Například při pokusu o čtení neexistující lokace v paměti vyskočí okno s touto informací a s kódem chyby.
Při skončení programu nebo zastavení přes signál se zobrazí okno upozorňující uživatele o této skutečnosti.
Některé zprávy vyžadují rozhodnutí uživatele. Jeden z případů je selhání funkce čekání na zastavení sledovaného procesu, kde uživatel má možnost opakovat tento pokus nebo proces zastavit. Druhý nastane, pokud uživatel chce vybrat nový soubor a starý program stále běží.
Dialogové zprávy zvyšují interaktivitu s uživatelem a zamezují přehlédnutí zásadních informací.

### Nastavení

Uživatel může upřesnit nastavení aplikace v konfiguraci. Ta je psána v textové podobě ve formátu *toml*[_] a nachází v adresáři "~/.config/tbd/config.toml", kde '~' značí domovský adresář uživatele. Je nutné podotknout, že tento soubor aplikace sama nevytváří.
Nastavení se dělí na několik částí. "Window" upřesňuje pozici a velikost okna při spuštění a také barevné schéma aplikace. V sekci "layout" můžeme nastavit rozložení a otevřené panely při spuštění. V "layout.panes" si můžeme vybrat, které panely chceme používat. Můžeme dokonce mít více panelů stejného typu. Poslední část se jménem "feature" slouží k používání experimentálních funckí. Aktuálně se jedná pouze o funkci, která zajišťuje správné rozbalování funkcí u programů, které byly psány v *Rustu*.
Není nutné specifikovat všechna nastavení, zbytek se automaticky doplní výchozími hodnotami. Výchozí nastavení je k nalezení v souboru "code/assets/config.toml". Zde jsou také vypsané všechny možnosti a další popis jednotlivých položek.

## Typický postup při používání

### Po spuštění

Po otevření hlavního okna si můžeme přemístit panely tam, kde nám vyhovují a upravit si jejich velikost. Poté můžeme vybrat soubor přes tlačítko na *toolbaru* vlevo.

### Vybrání programu a příprava na lazení

Vybereme spustitelný soubor a počkáme na jeho zpracování. Poté co se načte si můžeme prohlédnout *zdrojový kód* a umístít *breakpointy* na problematická místa. Je také dobré si umístit jeden někam na začátek kódu funkce "main". Poté můžeme spustit program přes tlačítko vlevo panelu "Control".

### Začátek a průběh lazení

Zobrazí se nám panely "Memory", "Registers", "Terminal" a "Assembly". Už teď si můžeme prohlížet informace o procesu. V této fázi není doporučeno používat tlačítko pro "Krok ve *zdrojovém kódu*". Můžeme tedy nechat kód běžet, až narazí na nějaký z *breakpointů*. Když se tak stane, můžeme si prohlédnou hodnoty lokálních proměnných, výstup z terminálu a přidat nebo odebrat některé *breakpointy*. *Breakpointy* bychom nikdy neměli umisťovat pokud kód právě běží. Na to máme možnost si kód pozastavit (2. tlačítko panelu "Control", když kód zrovna běží). Také bychom neměli pozastavovat kód, pokud právě čte z terminálu. Takto pokračujeme dál, tak jak potřebujeme.

### Ukončení programu

Pokud kód skončí, zjistíme to přes dialogové okno a dozvíme se hodnotu konečného kód (anglicky "exit code"). Program můžeme ukončit i sami přes tlačítko "Zastavení" nebo "Ukončení" (ukončení posílá signál SIGKILL). Stejný program můžeme zkusit spustit znovu nebo klidně nahrát nový. Pokud byl program ukončen signálem, tak je nám sdělen jaký signál to byl.

# Seznam použitých informačních zdrojů
