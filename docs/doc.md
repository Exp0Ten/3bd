# Cíle práce

Cílem mé práce bylo vytvořit debugger s grafickým uživatelským rozhraním. Program umí ladit kód kompilovaných jazyků, a tak pomáhá uživateli při hledání chyb v kódu. Nejdůležitější funkce lazení pro mě byly: možnost pozastavení programu, získání aktuální pozice v kódu, zobrazení stavu, ve kterém se nachází, čtení jeho paměti a zajištění komunikace skrze standardní vstup a výstup.
Má snaha byla hlavně vytvořit software, který je příjemný pro používání, dostatečně přizpůsobitelný a v ne menší řadě dále rozvíjitelný.
Pochopitelně jsem také chtěl získat další zkušenosti v oblasti operačního systému Linux, nízkoúrovňového programovaní a programovacího jazyka Rust.

# Způsoby řešení a použité postupy

## Návrh a obecná architektura

Program je navržený jako aplikace s grafickým prostředím. To znamená, že nemá daný postup spuštění, ale reaguje na vstup a příkazy uživatele. Jednotlivé moduly tedy spíše plní funkci knihoven, nežli jednoduše rozdělení kódu do několika částí.
Jádrem aplikace je proto uživatelské rozhraní. To samozřejmě má svoji strukturu **App**, ve které se ukládá jeho stav a všechny informace potřebné k zobrazování grafiky. Pro zbytek dat, které nesouvisí s funkcí aplikace, jsem použil globální proměnné. O ně se stará soubor "data.rs".
[image window::App] a [image data::Globals]

Tento přístup jsem zvolil kvůli optimalizaci a přehlednosti. Funkce používají méně parametrů a získal jsem možnost se vyhnout kopírování dat kvůli správě referencí.
Modul "dwarf.rs" je souborem funkcí, které zpracovávají informace k lazení. Jedná se o výpočetní komplex a mohl by být použit samostatně nebo v podobných projektech. Je zásadní pro lazení kódu na vyšší programovací úrovni.
Soubor "trace.rs" slouží k ovládání celého programu. Obsahuje funkce, které najdeme ve naprosté většině debuggerů. Také funguje jako most při lazení mezi grafikou a procesy chystající data v pozadí. Soubor "object.rs" je jeho menším doplňkem, který se primárně stará o komunikaci se systémem.
Nakonec je celý program navržen, aby fungoval jako samostatný spustitelný soubor. Struktura **Asset** obsahuje všechny externí soubory, které jsou zkompilované přímo do kódu.

### Průběh programu

Při spuštění programu se nejprve načte konfigurace. Přečte se nastavení uživatele a doplní se o výchozí hodnoty, popřípadě se použije výchozí nastavení jako takové.
Poté se spustí samotná aplikace a otevře se hlavní okno. Po vybrání souboru dochází k jeho zpracování. Nejdříve se načtou všechny sekce s informacemi pro lazení a zavolají se funkce **load_source** a **parse_functions**, které vytvoří data o *zdrojovém kódu* a funkcích. Toto předchystání dat snižuje výkon potřebný při samotném lazení.
Při zahájení lazení se otevře *pseudoterminál* a spustí se lazený program. Přečtou se také informace o tzv. *paměťovém rozložení* (anglicky "memory maps") a otevře se paměťový soubor procesu.
V průběhu se vyhodnocují *operace* (pod typem **Operation**) a signály, které proces obdrží. Vnitřní data se mění pouze při pozastavení, kdy se přepočítává většina informací o aktuálním stavu. Grafika tak při běžném používání nevyžaduje moc výkonu.
Pokud je proces zastaven, popřípadě změňen vybraný soubor, tak se uvolňuje paměť od dat, která již nejsou potřebná.

## Sledování procesu

Pro sledování se používá systémové volání *ptrace* [_] (anglicky "process trace"). Soubor "trace.rs" obsahuje funkce, které obalují toto sysémové volání z knihovny *nix*. Při selhání některé z nich se také objeví dialogové okno s chybou.

### Spouštění programu

K lazení programu potřebujeme, aby proces zavolal systémové volání *ptrace* s vlajkou "TRACEME". Tím prakticky říká nadřazenému programu, že může sledovat průběh jeho spuštění. Bez tohoto bychom potřebovali administrátorské pravomoce pokaždé, kdy aplikaci spouštíme.
Já ve svém kódu používám následující postup. Pomocí funkce *fork* nejdříve rozdělím proces na dva identické (ale se stejnou pamětí, takže nedochází ke zbytečnému kopírování dat). Poté v podřazeném z nich se zavolá zmiňovaná vlajka a pak se zavolá program, který chceme ladit.
Také se propojí jeho standardní vstup a výstup s *pseudoterminálem*.

### Ovládání průběhu

Proces vždy buď běží, nebo je pozastaven. Zastaví se když narazí na *breakpoint* nebo obdrží signál. Signály jsou zpracovávány funkcí **handle**, která se stará o aktualizaci dat o stavu programu.
*Breakpointy* jsou instrukce v kódu o velikosti bajtu, které vyvolají přerušení programu. Operační systém poté zachytí toto přerušení a pošle nadřazenému procesu informaci o zastavení. Nahrazený bajt si musíme uschovat, abychom *breakpoint* také mohli odstranit. Já ve svém kódu vytvářím typ **Breakpoints**, který představuje kolekci **HashMap**. Ta používá jako klíče *normalizované* adresy a jako hodnoty nahrazené bajty. *Normalizovaná* adresa neznačí přesnou pozici v kódu, nýbrž odchylku od začátku paměti programu. Při vkládání poté přičítám adresu, která značí začátek této paměti. Toto umožňuje zadávat *breakpointy* i před tím, než je program vůbec spuštěn.
Pokud je proces pozastaven, můžeme ho nechat pokračovat nebo spustit pouze nadcházející instrukci, tzv. "krok". Při pokračování vždy musíme udělat "krok" před tím, než budeme zapínat *breakpointy* (jinak bychom naráželi pořád na ten samý). "Krok ve *zdrojovém kódu*" používá jinou funkci nežli "krok". Na každou adresu, která je spojena s řádkem v *zdrojovém kódu*, umístíme *breakpoint* a program necháme pokračovat. Toto děláme, protože nejsme schopni předvídat, jaký řádek je opravdu ten další (při volání funkce, smyčce a podobných).
[image trace::SourceStep]

### Asynchronní smyčky

Pro čtení standardního výstupu z *pseudoterminálu* se používá asynchronní smyčka. Ta je vytvořena pomocí volání asynchronní funkce, která čeká na standardní výstup. Když dostane data, tak je vrátí a zavolá se operace **Operation::Read**. Výstup procesu se zpracuje a uloží. Pokud funkce nevrátila chybu, tak se znovu stejná čtecí funkce zavolá a tím se vytvoří smyčka.
Podobná smyčka je použita při čekání na singál. Ta se liší v tom, že sama sebe nevolá, avšak je zavolána při každém pokračování ve spuštění programu.
Tyto smyčky využívají rozhraní **Task**, které knihovna *iced* používá pro vykonávání asynchronních funkcí.

## Zpracování informací k lazení

Většina kompilerů v dnešní době používá standard *DWARF* při vytváření informací pro lazení (tzv. "debug info"). Jedná se kolekci sekcí v spustitelných souborech *ELF*, nejčasti jsou označeny prefixem ".debug_". Program při načítání těchto sekcí nejprve testuje jejich přítomnost. Nejdůležitější informace jsou pro nás propojení *zdrojového kódu* s *kódem strojovým*, umístění funkcí v *strojovém kódu* a získávání hodnot proměnných.
Program je rozdělen do několika *kompilačních jednotek*. Každá představuje nějaký samostatný celek kódu, například zkompilovaný soubor (nebo soubory) *zdrojového kódu* nebo modul. Obsahují definice funkcí, typů, proměnných atd., název, kompilační složku, programovací jazyk a veškeré informace potřebné lazení této jednotky.

### Řádkový program a zdrojový kód

Každá *kompilační jednotka* má svůj tzv. *řádkový program* (anglicky "line program"). Ten obsahuje pseudokód, který specifikuje lokaci řádků *zdrojového kódu* v *strojovém kódu*.
Na získání dat z těchto programů používám struktury **SourceMap** a **LineAddresses** a funkci **load_source**. **LineAddresses** je kolekce **HashMap**, která uschovává strukturu **SourceIndex** pod klíčem *normalizované* adresy. **SourceMap** také představuje **HashMap**, která uschovává vektor (standardní typ **Vec**) souborů *zdrojového kódu* pod klíčem kompilační složky. Zmiňovaný **SourceIndex** je struktura obsahující řádek *zdrojové kódu*, kompilační složku a index do vektoru souborů, který dostaneme, když dosadíme kompilační složku do **SourceMap**. Jednotlivé soubory *zdrojového kódu* ukládá struktura **SourceFile**. Ta obsahuje název souboru, respektive relativní cestu k souboru, odkaz na kompilační jednotku a pak obsah samotného souboru. Obsah se však nahrává až za chodu k ušetření paměti. V nadhledu tedy **SourceMap** uschovává všechny soubory *zdrojového kódu* a **LineAddresses** ukládá páry adres a pozic ve *zdrojovém kódu*.
Funkce **load_source** obě tyto struktury vytváří a pak ukládá do globálních proměnných. Informace získává skrze iteraci přes každou *kompilační jednotku* a poté vnitřní iteraci přes každou instrukci zmíněného pseudokódu. Ze všech instrukcí představující pozici v kódu se poté získá adresa, odpovídající řádek a soubor, ve kterém se řádek nachází. Ze souboru se poté dynamicky určí jeho kompilační složka. Pokud cesta k souboru používá relativní umístnění, přiřadí se výchozí kompilační složka celého programu. Pokud se jedná o absolutní cestu, tak algoritmus otestuje, jestli se část neshoduje s cestou výchozí jednotky. Pokud ano, tak se mu tato výchozí cesta přiřadí a přepočítá se relativní cesta k souboru. Jinak se použije absolutní cesta adresáře jako kompilační složka.
[image dwarf::load_source()]

Tento proces zajišťuje větší přehlednost v ukládání *zdrojového kódu* a umožňuje jednoduší manipulaci s výchozí kompilační složkou. Poté se adresa a řádka uloží do **LineAddresses** a přidá se nový **SourceFile** do **SourceMap**, pokud se zde ještě nenachází. Původně jsem chtěl použít místo **SourceIndex** pouze referenci na soubor v **SourceMap**. To však vyžadovalo tvorby *statické* reference, protože opouští funkci skrze globální proměnné. A při používání typu **Mutex** pro globální proměnné nelze statickou referenci vytvořit.

### Informace o funkcích

Každá funkce má v sekci ".debug_info" záznam pod svou *kompilační jednotkou*. Záznam funkce obsahuje často název, lokaci v *strojovém kódu*, typ návratové hodnoty a také pod sebou uschovává deklarace lokálních proměnncýh. Abychom jednodušeji hledali mezi těmito funkcemi, tak je předem zpracováváme do struktury **FunctionIndex**. O čtení těchto informací se stará funkce **parse_functions**.
**FunctionIndex** slouží k zjištění aktuální funkce pomocí adresy a obsahuje tři kolekce **HashMap**. První schraňuje odkazy k funkcím pod klíči počáteční adresy funkce. Druhá ukládá intervaly všech funkcí v kompilační jednotce. Poslední obsahuje jména nadřazených celků ke každé funkci. Toto rozdělení do několika **HashMap** umožňuje rychlejší získávání dat při práci s většími projekty.
Většina záznamů má atributy "low_pc" a "high_pc", které udávají rozsah funkce. Zbylé používají sekci ".debug_ranges", zpravidla pokud funkce obsahuje funkci vloženou. Podobně, každá deklarace bez rozsahu má zpravidla svou specifikaci, která obsahuje pro nás potřebný rozsah.
Funkce *parse_functions* čte tedy pouze rozsahy funkcí v *strojovém kódu*. Iteruje přes všechny kompilační jednotky a v průběhu si ukládá v řadě názvy předchozích záznamů. Pokud záznam představuje deklaraci, neobsahuje rozsah funkce, je ale nutné si uložit název nadřazeného záznamu. Tento název je poté použit pro specifikaci funkce (například pokud ve dvou souborech najdeme funkci se stejný názvem).

### Callstack

Při zastavení na řádce v kódu se zpravidla nacházíme v nějaké funkci. Neznáme však lokální proměnné a nevíme, jak jsme se na tuto pozici dostali. Proto rozbalujeme řadu volání funkcí, neboli tzv. *callstack*. *Callstack* získáme tak, že zjistíme v jaké funcki se nacházíme, vyhledáme si informace, které umožňují rozbalení aktuálního stavu, tzv. *unwind information*, přečteme si pro nás zajímavé informace (lokální proměnné, parametry a další) a vyhodnotíme předchozí stav. Tento proces poté opakujeme dokud se nedostaneme do hlavní funkce, která je označená symbolem "main" nebo je specifikována v záznamech funkcí.
Hlavní částí zmíněných *unwind information* je tzv. "cannonical frame address" (pod zkratkou CFA). *CFA* určuje pozici na *stacku*, která představuje základní adresu pro aktuální funkci. Od ní se také odvíjí pravidla na získání předchozího stavu a tyto informace obsahuje sekce ".eh_frame". Díky *CFA* tak můžeme získat adresu, kam se funkce vrátí až dokončí svou operaci. Tuto adresu pak používáme pro rozbalení další funkce. *CFA* je také nezbytná pro čtení lokálních proměnných.
O rozbalení *callstacku* se stará funkce **unwind**, která do struktury *CallStack* ukladá vektor jednotlivých funkcí. Využívá také prakticky všechny předchystané informace k lazení. Nejdříve získá přes řádek (získaný pomocí adresy) *kompilační jednotku*. Ta nám společně s adresou umožňuje najít funkci. Poté se získají základní informace o funkci (jméno, typ návratové hodnoty, jméno nadřazeného záznamu) a vypočítá se hodnota *CFA*. Přes základní adresu se získají lokace parametrů a lokálních proměnných a načteme další informace o nich. Poté se rozbalí předchozí hodnoty registrů a získáme stav před zavolání aktuální funkce.
Funkce **callstack** poté iteruje **unwind**, dokud se celý *callstack* nerozbalí.

### Rozbalování typů a zobrazování proměnných

Při rozbalování se ukládají informace o lokálních proměnných a parametrech. Jedná se o název, typ, lokaci, popřípadě konstatní hodnotu. Zpracováváme však pouze proměnné, které byly již v kódu deklarované (pomocí porovnání aktuálního řádku s atributem v záznamu proměnné). Typ zatím ponecháváme pouze ve tvaru odkazu na záznam v sekci ".debug_info".
Hned po vytvoření struktury **CallStack** se zavolá metoda **stack_lines**, která ze zpracovaných informací vytvoří řádky textu, které již můžeme zobrazit. To zejména šetří výkon grafiky, ale omezuje výstupní formát a původně jsem chtěl hodnoty vykreslovat přímo z jejich strukturní formy. Každý řádek obsahuje text a číslo hloubky, které používáme na vytvoření odsazení mezi řádky. Funkce **stack_lines** iteruje metodu **lines** struktury **Function**, ta zase metodu **lines** struktur **Variable** a **Parameter** atd.
Pro správné zobrazení hodnoty musíme znát typ proměnné nebo parametru. Na to používáme funkci **unwind_type**, která najde přes odkaz záznam typu a určí, o který se jedná (výčtový typ **TypeDisplay**). Všechny typy končí buď jako **BaseType**, **PointerType** nebo **EnumType**, jelikož ostatní typy vyžadují rozbalení typu, na který odkazují. Čtení z paměti získávající hodnoty proměnných se proto nachází pouze v metodách těchto typů.
Pro získání hodnot proměnných musíme tedy umět typy znovu rozbalit. Proto tato část kódu používá komplexní rekurzy na správné zobrazování hodnot. Prvně se rozbalí původní typ za tvorby typu **TypeDisplay**. Pak se na něm zavolá metoda **value**, která obecně volá metody **value** jednotlivých typů. Pokud se nejedná o zmíněné typy, tak se znovu zavolá funkce **unwind_type** tam, kde je potřeba a cyklus se opakuje.
[image dwarf::TypeDisplay] [image diagram]
**BaseType** používá atribut "encoding" na určení, jak danou hodnotu zobrazit. **PointerType** je vypsán vždy jako šestnáctkové číslo v ostrých závorkách. **EnumType** porovnává proměnnou s konstantními hodnoty *enumerátoru* a pak zobrazuje jeho název. **ModifierType** pouze přidá do textu své jméno a rozbalí další typ. **ArrayType** vypíše pouze své jméno a jednotlivé elementy vypíše podsebou. **StructType** funguje podobně jako **ArrayType**, ale nemá stanovený počet, nýbrž jednotlivé členy s vlastním typem. **TypeDef** narozdíl od ostatních typů jméno nepřidává, ale odstraňuje celý rozbalený typ , podle kterého byl definován, a zanechá pouze poslední slovo (zpravidla hodnotu). Pokud zobrazení jakéhokoliv typu selže, vypíše se na jeho místo '?'.

## Uživatelské rozhraní

Grafické rozhraní aplikace používá dvě hlavní funkce ze souboru "window.rs". Rozložení a vykreslování okna je dáno funkcí **view**, která volá funkce v souboru "ui.rs". Ty vždycky vyjímají ucelonou část grafiky pro větší přehlednost. Druhá funkce **update** se naopak stará o akce a zajišťuje interaktivitu uživatele s celým prostředím. K třídění používá typ **Message**. Ten se dělí na **PaneMessage**, starající se o jednotlivé panely, **LayoutMesage**, který má na starost rozložení, a **Operation**, ovládající laděný proces.

### Mřížka panelů

Pro silné grafické přizpůsobení používám modul **pane_grid** z knihovny **iced**. Ten rozděluje plochu do několika menší panelů, které lze přesouvat a měnit jejich velikost. Tento návrh dále rozšiřuji o postranní panel, které se dají schovat. Informace o uspořádání vlastní struktura **Layout**.
Rozložení se řídí těmito pravidly: každá plocha je buď rozdělena na dvě další podle vodorovné či svislé osy, nebo obsahuje panel. Pomocí těchto jednoduchých pravidel vytvářím pomocí metody **base** základní rozvržení celé plochy. To se řídí tzv. módem spodního panelu a otevřenými či schovanými postraními panely. Při schování postranního panelu se část plochy odebere a uloží do globální proměnné, abychom uložili vnitřní stav panelů. Proto se při změně postranních panelů zavolá funkce **layout**, která celé rozložení znovu vytvoří. 
Každé rozdělení také obsahuje poměr, podle kterého je plocha rozdělena. Při změně velikosti se tato hodnota mění. Kvůli jednodůchým pravidlům se ale také mění velikost vnitřních ploch a panelů. Proto ve funcki **resize** používám specifický postup, který pomocí výpočtů s poměry zajišťuje, že velikost postranních panelů zůstává stejná.
[image: diagram]
Na obrázku můžeme vidět příklad tvorby rozložení. Nejdříve se plocha rozdělí podle vodorovné osy za vzniku spodního panelu, poté podle svislé osy za vzniku levého panelu a nakonec znovu podle svislé osy za vzniku středu a pravé postranního panelu. Poté se do středu a každého postranního panelu vloží jednotlivé menší panely.

### Panely a data 

Panely jsou zobrazovány pomocí funkce **mainframe**. Zde se také volá funkce **pane_view*, která přesměrovává vykreslování daného typu panelu. K manipulaci se stavem panelů používáme funkci **pane_message**. Významné operace této funkce se týkají hlavně panelů **PaneCode** a **PaneMemory**.
Při vybrání souboru *zdrojového kódu* v **PaneCode** se asynchronně vytvoří seznam *breakpointů*. Původně jsem tuto operaci dělal při samotném vykreslování, ale vytvářela příliš velkou zátěž a prostředí se začalo sekat. Pokud vybraný soubor ještě nebyl načten, tak se program pokusí ho otevřít a přečíst.
**PaneMemory** používá velkou optimalizaci při načítání dat z paměťi. Při každém čtení se totiž přečtou 4KB a panel poté používá tato data a načítá nová pouze při přiblížení se ke hraně intervalu. Pokud jsme však blízko hranice přístupné paměti, čtení se nastaví na takovou hodnotu, abychom četli začátek nebo konec paměti. Posledním pravidlem je nečíst nová data, pokud poslední čtení používalo stejnou čtecí adresu (jelikož data již máme). Tento kód se nachází ve funkci **update_memory**.  
Některá data panelů jsou z jejich podstaty uschována v celkovém stavu aplikace (například standardní výstup terminálu). Pro panely **PaneCode** a **PaneStack** se využívá vnitřní stav, ale jejich obsah může být aktualizován. To vidíme zejména při sjetí k řádku v *zdrojovém kódu* při zastavení programu. U **PaneStack** používáme unikátní číslo, které značí poslední aktualizaci panelu. To je důležité, protože panel může být skrytý a poté se neaktualizuje. Číslo je při každé změně inkrementováno a pokud se čísla stavu aplikace a panelu nerovnají, tak se zobrazí tlačítko na nahrání aktuálních dat.

## Cizí kód

Nakonec bych rád vymezil externí části kódu, které jsem ve svém projektu použil.
Při své práci jsem v souboru "trace.rs" vycházel z tutoriálu "Writing a Linux Debugger", který popisuje psaní debuggeru pro Linux v jazyce *c++*.
Stejný postup jako zmíněný zdroj používám ve funckích **insert_breakpoint** a **remove_ breakpoint**, kde vyměňuji bajt v paměti lazeného programu[_], při operaci **Operation::Continue**, kde nejdřív udělám krok z aktuální pozice a pak až vkládám *breakpointy* a pokračuji ve spuštění programu[_], a ve funkci **handle**, ve které rozpoznávám naražení na *breakpoint*[_].
V souboru "dwarf.rs" jsem se na několika místech inspiroval příklady z dokumentace knihovny *gimli*. Specificky ve funcki **load_source**, kde jsem použil příklad uvedený pro čtení *řádkových programů*[_]. Ve funkci **load_dwarf** používám kód pro čtení sekcí *DWARF* pro vytvoření struktury **DwarfSections**[_]. A funkce **create_assembly** je napsána podle vzoru na stránkách dokumentace knihovny *iced-x86*[_].

# Programovací prostředky

## Programovací jazyk Rust, rustc a rust-analyzer

Pro tento projekt jsem zvolil programovací jazyk Rust kvůli jeho stabilitě, bezpečnosti a rychlosti. Mám v něm také nejvíce zkušeností a chci se v něm dál rozvíjet.
Pro samotné programování byly nejužitečnějšími nástroji kompiler tohoto jazyka, zvaný *rustc*, a doplněk *rust-analyzer* pro *VSC*. Dohromady zajišťovaly příjemné vývojové prostředí: doplňování a navrhování funckí a metod a podrobné vysvětlení chyb v kódu.

## Visual Studio Code (VSC)

Na psaní kódu jsem používal textový editor *VSC*. Pro účely programování mi vyhovuje z několika důvodů: 
- široká nabídka doplňků a jejich jednoduchá správa
- bohaté zvýrazňování v kódu
- možnost kompletního uživatelského přizpůsobení
- rychlé hledání v textu a funkce multi-kursoru

## GNU Make

V mém projektu se také objevuje několik souborů scriptovacího jazyka *GNU Make*. *GNU Make* zjednodušuje celý proces kompilace a používám ho jak pro samotný debugger, tak pro příklady *zdrojových kódů* ve složce "examples".

## Microsoft Copilot

Při práci na projektu jsem také v některých částech použil kód, popřípadě konzultaci, od generativní umělé inteligence Copilot. Rád bych tyto pasáže zde vyjmenoval a upřesnil:

V souboru "data.rs" jsem si nevěděl rady s globálními proměnnými. Copilot mi doporučil použití typu standardní knihovny **Mutex**, který zajišťuje jejich synchronizitu a bezpečnost čtení a zápisu. Také jsem zjistil jak spravovat statické reference, a to hlavně pro ukládání hrubého obsahu laděného programu.
V souboru "object.rs" jsem si nechal vysvětlit jak fungují pseudoterminály a jak z nich vytvořit standardní komunikaci s laděným kódem.
V souboru "dwarf.rs" mi Copilot vygeneroval makro, které implementuje methody převádějící bajty do různých typů čísel, abych nemusel vypisovat repetetivní kód (specificky manuální implementaci pro každý typ). Také funkce **align_pointer** pro vytváření *assemblerového kódu* byla navržená Copilotem.
A nakonec jsem potřeboval vysvětlit proces a implementaci získávání návrátové adresy (anglicky *return address*) funkcí. Copilot mi nabízel vypočítávání pozice s touto adresou přes odchylku, já jsem však nakonec použil vnitřní funkci knihovny *gimli*, která odvozuje tyto hodnoty přes informace k lazení.

Všechny zmíněné části kódu jsou označené komentářem "//AI".

## Použité knihovny

### nix

Knihovna *nix* je nádstavbou Standardní knihovny pro jazyk C (známá také jako "libc"). Použil jsem ji hlavně pro moduly *ptrace*, *process* a *term*. *Ptrace* umožňuje sledování procesů, z *process* jsem použil funkci *fork* pro spuštění nového podprocesu a *term* byl nezbytný pro vytvoření pseudoterminálu na standardní komunikaci. [_]

### iced

Knihovna *iced* přináší prostředí pro jednoduchou tvorbu grafických aplikací. Ačkoliv je stále ve vývoji, nabízí bohatou škálu přehledných funkcí pro tvorbu grafiky a interakce s uživatelem. Taky jsem s ní s měl trochu zkušeností. [_]

### gimli

Knihovna *gimli* spravuje informace pro lazení, zejména ty pod standardem *DWARF*. Jedná se o velmi silný modul na *parsování* těchto dat. Používám ji zejména pro zobrazování *zdrojového kódu* a sledování lokálních proměnných. [_]

### Další

Mimo hlavní knihovny jsem dále použil:
- *iced-x86* jako *disassembler strojového kódu* pro lazení i na nižší úrovni [_]
- *toml* pro nahrávání a *parsování* konfiguračních souborů [_]
- *rfd* na vybírání programu skrze průzkumníka souborů a také komunikaci s uživatelem přes tzv. dialogová okna [_]
- *rust-embed* pro kompilaci všech externích souborů (tzv. *assets*) do výsledného spustitelného kódu (vytvoření tzv. *single-file binary executable*) [_]
- *std* jako standardní knihovnu jazyka *Rust* pro obecné funkce, struktury a další [_]

Široký seznam knihoven byl pro mě bližší nežli závislost projektu na externích programech.

## Ikony Adwaita

V práci také používám soubor ikon *Adwaita* [_] vytvořený pod projektem *GNOME*. Ve složce "code/assets/icons" se s použitými ikonami nachází soubory "LICENSE" a "README.md", které upřesňují licencování této grafiky. Všechny ikony, vyjma souboru "TBD.svg", licencuji pod licencí "GNU Lesser General Public License v3.0". Ta je kompatibilní s tou, která licencuje celý můj projekt ("GNU General Public License v3.0").

# Zhodnocení dosažených výsledků

Ačkoliv jsem nedosáhl všech mých původních představ, věřím, že jsem zadání splnil a osobně jsem s prací velmi spokojený. Myslím si, že jsem dokázal vytvořit software, který je funkčí, přizpůsobitelný a rozvíjitelný. Rád bych však vyjmenoval některé nedostatky, které v práci cítím.
Chybí mi podrobnější barevné zvýrazňování v aplikaci, hlavně v panelech **PaneCode** a **PaneStack**. Stejně tak jsem zjistil při zkoušení dalších vestavěných barevných schémat z knihovny *iced*, že u některých z nich je text v programu špatně čitelný.
Dále mi nevyhovuje funkce panelu **PaneAssembly**, jelikož se nedá prohlížet *assemblerový kód* daleko od aktuální pozice v kódu.
Určitě bych panelu **PaneStack** přidal možnost zobrazení hodnot proměnných v různých formátech a také funkci připnutí některých řádků pro větší přehlednost.
A nakonec mi chybí klávesové zkratky ovládající aplikaci a vybírání souboru přes argumenty příkazu na spouštění aplikace.
Program plánuji nadále vylepšovat a rozšiřovat, hlavně za účelem odstranění zmíněných nedostatků.

# Instalace

Ve složce *build* se nachází soubor *Makefile*, který obsahuje script pro program *make*. Stačí tedy v tomto adresáři zadat příkaz "make" (nebo "make all") a celý program se zkompiluje a vytvoří se zde spustitelný soubor "tbd". Ten se dá poté spustit pomocí "./tbd". (Pro instalaci na použití kdekoliv v systému můžeme přesunout soubor do "/bin" nebo "/usr/bin" nebo přidat cestu k souboru do systémové proměnné "$PATH".)
Program se dá spustit i mimo prostředí terminálu.

## Nároky a kompatibilita

Program podporuje pouze operační systém Linux, měl by však fungovat na většině distribucí, v desktopových prostředích typu *x11* i *wayland*.
Výsledný soubor má 21MB, a z mého testování program nikdy nevyžadoval více než 120MB paměti. Celková zátěž paměti se však zvyšuje dvojnásobně s velikostí lazeného kódu, protože jeho obsah je načten pro zpracování a pak ještě spuštěn. Debugger vyžaduje největší výkon při zvolení souboru na zpracování dat. Při velkém množství informací k lazení může tento proces trvat i několik vteřin, například u programů psaných v *Rustu*. K zpomalení grafiky také může dojít, pokud debugger zobrazuje velký text *zdrojového kódu*.
Program využívá tzv. *procfs*. Jedná se správu běžících procesů skrze souborovým systém. Pro správný chod programu je nutné, aby prostředí toto rozhraní podporovalo. O tom se můžeme ujistit promocí příkazu "test -d /proc && echo true" (program vypíše "true", pokud najde složku tohoto rozhraní).
Všechen kód byl vyvíjen a důkladně testován v distribuci Debian 12 Bookworm a v prostředí *wayland*, specificky v KDE Plasma. Úplná kompatibility byla také zajištěna pro Debian 13 Trixie.

## Externí závislosti

Na kompilaci kódu je potřeba sada nástroju pro Rust s názvem *rustup*. Manuální instalace použitých knihoven není potřebná, o ty se stará nástroj "cargo" z této sady nástrojů. [_]
Dále pro dialogová okna je nutný program *zenity*. Tato závislost není povinná, je ale silně doporučená. [_]
Projekt používá *GNU Make* na jednodušší kompilaci a jeho instalace je pro zajištění správného fungování programu potřebná. [_]
Pro kompilaci příkladů kódu ve složce "examples" jsou vyžadovány kompilery *GNU C Compiler*[_] a *GNU C++ Compiler*[_], také známé jako *gcc* a *g++*.
A v neposlední řadě je nutná *Knihovna GNU C*, také známa jako *glibc*. Na *Debianu* se specificky jedná o balík *libc6*. [_]
Popis instalace těchto závislostí pro distribuce Debian a Ubuntu se nachází v souboru "DEPENDENCIES.md".

# Ovládání

## Popis uživatelského rozhraní

Celé uživatelské rozhraní se skládá ze tří částí. Tou nejdůležitější je hlavní okno, přes ktére se program ovládá, a kde se zobrazují všechny informace při lazení kódu. Další jsou menší dialogová okna, která sdělují uživateli zásadní zprávy v průběhu používání. Třetí z nich je nastavení skrze konfigurační soubory. 

### Okno, toolbar, panely

Okno se dělí na *toolbar* nahoře, *statusbar* dole, a *mainframe* uprostřed.
*Toolbar* obsahuje vlevo tlačítko na vybrání souboru a vpravo tlačítka na otevření či schování postranních panelů.
*Statusbar* obsahuje obecné informace jako název vybraného souboru, identifikační číslo procesu (tzv. "PID"), stav procesu a popřípadě na jakém řádku *zdrojového kódu* je proces zastaven.
*Mainframe* obsahuje panely, které zobrazují veškeré informace o procesu a umožňují lazení kódu. Tyto panely můžeme přesouvat, měnit jejich velikost, a tak si přizbůsobit celé grafické prostředí.
Každý panel má svůj titulek se jménem a pod ním svůj obsah. V horní části obsahu se soustřeďují interaktivní struktury, například tlačítka, výběry ze seznamu a další. Většina panelů má své vnitřní hodnoty, a proto můžeme používat i více panelů stejného typu zároveň.

### Panely podrobně

Panelů je celkem 8 a každý plní jinou funkci.

Panel "Control" obsahuje tlačítka na ovládání lazeného procesu. Zleva doprava jdou následovně: "Spustit/Zastavit", "Pokračovat/Pozastavit", "Krok", "Krok v *zdrojovém kódu*", "Ukončit" a "Poslat Signál". Úplně vpravo poté můžeme vybrat signál, který chceme procesu poslat. "Krok" znamená spustit další instrukci, zatímco "Krok v *zdrojovém kódu*" spustí program a zastaví se jakmile narazí na pozici, která je spojená s řádkem v *zdrojovém kódu*.
Panel "Memory" zobrazuje obsah paměti programu. V poli můžeme zadat adresu v šestnáctkovém nebo desítkovém zápisu. Tlačítko úplně vpravo mění počet zobrazovaných bajtů na řádek. Zbylá tlačítka specifikují formát, ve kterém jsou jednotlivé bajty zobrazeny. V těle panelu se pak nachází řádky s adresami a bajty. Adresa vždy ukazuje pozici prvního bajtu v řádku. Při použití kolečka na myši se můžeme pohybovat nahoru a dolu.
Panel "Code" zobrazuje zdrojový kód lazeného programu. Nahoře můžeme vybrat tzv. kompilační složku a soubor. Tlačítko vpravo udává, jestli chceme panel aktualizovat, a tak sledovat aktuální pozici v kódu. Pozice, na které právě jsme, je zobrazena přes zvýrazněné číslo řádku. Pokud řádek má k sobě přiřazenou adresu, můžeme na něj dát *breakpoint*. Tlačítka na *breakpoint* se nachází vlevo od čísla řádků.
Panel "Registers" zobrazuje hodnoty registrů procesoru. Tlačítka nahoře mění číselnou soustavu, ve které jsou hodnoty zobrazovány.
Panel "ELF Info" vypisuje informace, které nalezneme v hlavičce *ELF* souborů. Jedná se například o cílený operační systém a architekturu, vstupní adresu kódu a další.
Panel "Terminal" funguje jako vnitřní terminál pro stadardní komunikaci s lazeným programem. Nahoře je okno se stadardním výstupem, dole je pole pro standardní vstup. Aktuální pozice kursoru je zobrazována znakem '_'. Pro poslání standardního vstupu je nutné zmáčknout klávesu "Enter".
Panel "Assembly" vypisuje assemblerový kód okolo aktuální pozici. Ta je vyznačena zvýrazněnou adresou vlevo. Instrukce jsou zobrazovány ve formátu assembleru *nasm*. Jsou vypsány také bajty každé instrukce a vedle adres najdeme tlačítka na *breakpointy*.
Panel "CallStack" vypisuje tzv. *callstack*, neboli seznam funkcí podle toho, jak byly postupně volány. U každé funkce pak vypisuje deklarované proměnné a jejich typy a hodnoty. Řádky značící zavolání funkce jsou zvýrazněny. "0" znamená prvně zavolaná funkce, zpravidla "main". Pole funkcí, struktur a řetězců jsou zobrazovány s odsazením pro lepší čitelnost. Vedle řádků se proto nachází tlačítka na schování či rozbalení odsazených polí. Při aktualizaci je vždy obsah aktuální funkce rozbalen zatímco všech ostatních schován.

### Dialogová okna

Dialogová okna sdělují důležité informace uživateli. Jedná se o chyby při lazení kódu, informace o průběhu spuštěného procesu a varování při nestandardních situacích.
Zprávy o chybách vždy zobrazí příčinu chyby a případně další informace o selhání některé z funkcí. Například při pokusu o čtení neexistující lokace v paměti vyskočí okno s touto informací a s kódem chyby.
Při skončení programu nebo zastavení přes signál se zobrazí okno upozorňující uživatele o této skutečnosti.
Některé zprávy vyžadují rozhodnutí uživatele. Jeden z případů je selhání funkce, která čeká na zastavení sledovaného procesu. Uživatel má možnost tento pokus opakovat nebo proces zastavit. Druhý nastane, pokud uživatel chce vybrat nový soubor a starý program stále běží.
Dialogové zprávy zvyšují interaktivitu s uživatelem a zamezují přehlédnutí zásadních informací.

### Nastavení

Uživatel může upřesnit nastavení aplikace v konfiguraci. Ta je psána v textové podobě ve formátu *toml*[_] a nachází se v adresáři "~/.config/tbd/config.toml", kde '~' značí domovský adresář uživatele. Je nutné podotknout, že tento soubor ani adresář aplikace sama nevytváří. (stačí vytvořit složku "tbd" v adresáři "~/.config/" a v ní pak vytvořit soubor "config.toml")
Nastavení se dělí na několik částí. Nastavení "window" upřesňuje pozici a velikost okna při spuštění a také barevné schéma aplikace. V sekci "layout" můžeme nastavit rozložení a otevřené postranní panely při spuštění. V "layout.panes" si můžeme vybrat, které panely chceme používat. Můžeme dokonce mít více panelů stejného typu. Poslední část se jménem "feature" slouží k používání experimentálních funckí. Aktuálně se jedná pouze o funkci, která zajišťuje správné rozbalování řady *callstacku* u programů, které byly psány v jazyce *Rust*.
Není nutné specifikovat všechna nastavení, zbytek se automaticky doplní výchozími hodnotami. Výchozí nastavení je k nalezení v souboru "code/assets/config.toml". Zde jsou také vypsané všechny možnosti a další popis jednotlivých položek.

## Typický postup při používání

Programy, které chceme sledovat bychom měli zkompilovat tak, aby se v spustitelném souboru vytvořili informace pro lazení. Nejčastěji to znamená přidat kompileru vlajku "-g" při zadávání příkazu ke kompilaci. Tyto informace se vytvoří nehledě na přítomnosti jiných vlajek, například optimalzačních. Pro nejlepší průběh lazení je však doporučeno nepřidávat jiné vlajky nežli pro lazení.

### Po spuštění

Po otevření hlavního okna si můžeme přemístit panely tam, kde nám vyhovují a upravit si jejich velikost. Poté vybíráme soubor přes tlačítko na *toolbaru* vlevo.

### Vybrání programu a příprava na lazení

Vybereme spustitelný soubor a počkáme na jeho zpracování. Poté co se načte si můžeme prohlédnout *zdrojový kód* a umístít *breakpointy* na problematická místa. Je obecně dobré si umístit alespoň jeden někam na začátek kódu funkce "main". Poté můžeme spustit program přes tlačítko vlevo na panelu "Control".

### Začátek a průběh lazení

Zobrazí se nám obsah panelů "Memory", "Registers", "Terminal" a "Assembly". Už teď si můžeme prohlížet informace o procesu. Program je po spuštění zastaven. Není doporučeno používat tlačítko pro "Krok ve *zdrojovém kódu*" dokud nevstoupíme do hlavní funkce, zpravidla "main". Můžeme tedy nechat kód běžet, až narazí na nějaký z *breakpointů*. Když se tak stane, můžeme si prohlédnout hodnoty lokálních proměnných, výstup z terminálu a přidat nebo odebrat některé *breakpointy*. *Breakpointy* bychom nikdy neměli umisťovat pokud kód právě běží. Na to máme možnost si kód pozastavit (2. tlačítko panelu "Control", když kód zrovna běží). Také bychom neměli pozastavovat kód, pokud program právě čeká na standardní vstup z terminálu. Takto pokračujeme dál, tak jak potřebujeme.

### Ukončení programu

Pokud kód skončí, zjistíme to přes dialogové okno a dozvíme se hodnotu konečného kód (anglicky "exit code"). Program můžeme ukončit i sami přes tlačítko "Zastavení" nebo "Ukončení" (ukončení posílá signál SIGKILL). Stejný program můžeme zkusit spustit znovu nebo klidně nahrát nový. Pokud byl program ukončen signálem, tak je nám sděleno, jaký signál to byl.

## Poznámky

Lazený program by měl být vždy ukončen před zavřením samotné aplikace. Při ukončení debbugeru dojde totiž k odpojení podprocesu a ten pak pokračuje sám v normální chodu.
Aplikace se také může dostat do stavu, kdy neodpovídá. Naprosto nejpravděpodobnější příčina je, pokud je právě zobrazeno dialogové okno. Toto není chyba a jedná se o účelnou implementaci.

# Seznam použitých informačních zdrojů
