# Kompilery a interprety

Kompilery a interprety tvoří základní kámen pro moderní programování. Oba typy softwaru umožňují spuštění abstraktního kódu, který se řídí pravidly daného programovacího jazyka, a tak zjednodušují celý proces programování.

## Kompilery

Kompilery jsou programy, které překládají *zdrojový kód* do *cílového kódu*. Výstupní jazyk bývá zpravidla nižší programovací úrovně nežli ten vstupní [1]. Může to být například *strojový kód*, jazyk *Assembly* nebo *LLVM IR*.
Prvně probíhá tzv. *parsing*, neboli čtení kódu uživatele a jeho syntaktická analýza; sestavují se funkce, proměnné, konstanty apod. Poté dochází k samotnému překládání: každý výraz ve vstupním jazyce má svůj definovaný zápis v jazyce výstupním. Takto se vytvoří hrubý náčrt výsledného kódu. Nakonec se podle logických a optimalizačních pravidel vytvoří výsledné uspořádání (případně se vynechají nepotřebné části), dosadí se číselné definice za pojmenovaní (anglicky *labels*) a výsledek je vypsán do souboru nebo standardního výstupu.

Nejzákladnějším příkladem kompilerů je *assembler*, kterému stačí překládat uživatelsky čitelné instrukce do *strojového kódu* a dosazovat čísla. Pokročilejší kompilery vyšších jazyků však umí mnohem více: makra, direktivy (příkazy), obecně definované struktury (tzv. *generics*) a jiné. Například *rustc* analyzuje validitu obsahu proměnných a upozorňuje na nestabilní funkce kódu.

Kompilace přímo do *strojového kódu* vyžaduje znalost architektury cílového procesoru, a proto se dnes už příliš nevyskytuje. Místo toho se používají podpůrné nástroje, například *LLVM*, které používají obecný zápis k vytvoření samotného spustitelného souboru. Nové kompilery tak nemusí znovu implementovat všechny architektury.
Často se průběh kompilace dá pozměnit: specifikace úrovně optimalizace, vytvoření informací k lazení kódu a další.

[1] wiki: Compilers

## Interprety

Interprety umí spustit *zdrojový kód* i bez předchozí kompilace [2]. Podobně jako u kompilerů se v první fázi kód *parsuje*, avšak ne jako celý soubor, nýbrž po částech (většinou po řádcích). Každá část představuje nějaký příkaz a ten se musí interpretovat (od toho také pochází název "interpret").

Interpetace můžeme dosáhnout několika způsoby. Nejjednodušší z nich je volání funkcí na základě shody textu. Tato metoda je rychlá a využívaná skriptovacími programovacími jazyky (např. *bash* [3]). Má však nevýhodu velikosti a paměťové zátěže samotného interpreta, protože všechny funkce musí být předem zkompilované.
Dalším způsobem je kompilace příkazu do pseudokódu, většinou se jedná o takzvaný *bytecode*. Pseudokód je pak spuštěn pomocí obdobných pravidel jako v předchozí metodě, používá však *virtuální stroj*, který kód vyhodnocuje jako instrukce, takže výrazně rychleji a s menší zátěží na paměť [2]. Tato implementace je nejpoužívanější mezi interprety a najdeme ji například v jazyku *lua* a *python* [4].
Jako nejrychlejší řešení bývá považován *JIT*, neboli překládání do *strojového kódu* za běhu. Po *parsování* je kód rovnou nebo s mezikrokem zkompilován a spuštěn přímo na úrovni procesoru. Využívá tak vysoké rychlosti, kterou mají kompilované jazyky, je však velmi náročný na implementaci. [5]

Interprety se mimo spouštění uživatelského kódu využívají na *virtualizaci*, *emulaci* nebo také jako *terminály* a *příkazové řádky*. [2]

[2] wiki: Interpretation
[3] wiki: Bash Unix Shell (§Execution)
[4] wiki: Python
[5] wiki: JIT

## Silné a slabé stránky v porovnání

Zasádní výhody plynoucí z kompilace jsou rychlost a efektivita. Kompiler využívá více času na zpracování a vyhodnocení zdrojového kódu, a tak může odstranit nepoužité části kódu a optimalizovat celkový průběh výsledného kódu. Taky umí vyprodukovat samostatný spustitelný soubor, který nevyžaduje externí programy nebo knihovny. Navíc díky kompilaci můžeme spojit kód několika jazyků do jednoho programu.
Na druhou stranu kompilery většinou potřebují přesné definice v kódu. Jsou tedy méně dynamické a jejich výsledná optimalizace záleží z velké části na *zdrojovém kódu*. Kompatibilita výsledného programu mezi systémy a architekturami je také velmi omezená a zpravidla vyžaduje jeho *rekompilaci*, nebo dokonce přepsání *zdrojového kódu*.

Interprety jsou používány zejména pro svou jednoduchost, volnost a interaktivnost. Kód interpretovaných jazyků bývá přehlednější a jednodušší na tvorbu, například díky dynamickému typování proměnných. Také dovolují spuštění kódu, který má v sobě logické či syntaktické chyby, což může, ale nemusí bý výhodou. Mimo jiné vytváří kompatibilní prostředí: pokud je intepret implementovaný pro danou architekturu nebo operační systém, můžeme spustit náš *zdrojový kód* i zde. Interprety navíc vybírají optimální funkce a optimalizace tedy nespoléhá pouze na programátorovi. Avšak neumí přeskočit nepotřebné části kódu.
Hlavní nevýhoda interpretace je již zmiňovaná časová, popřípadě paměťová náročnost. Kvůli *parsování* trvá spouštění déle a jazyky si většinou nemohou dovolit jinou metodu čištění paměti nežli tzv. *garbage collector* (průběžné čištění nepotřebné paměti). Pro spuštění kódu je také pokaždé potřeba přítomnost interpreta a nelze ho spustit v prostředí, které není daným interpretem podporované.

## Praktická využití

Oba typy softwaru jsou kromě klasického programování využívány pro různé účely.

Další typy kompilerů jsou například: [1]
- *cross-compilers* - kompilují kód do jiných architektur, než ve kterě jsou spuštěni
- *dekompilery* - překládající kód z nižší úrovně do vyšší úrovně programovacích jazyků
- *source-to-source kompilery* - překládající kód z jednoho jazyka do druhého podobné úrovně
- *bootstrap kompilery* - dočasné kompilery pro kompilaci stabilnějších a lepších kompilerů

Interprety dále umožňují například: [2]
- *virtualizaci* - použítí jako virtuální stroj, spouštějící *strojový kód* jiné architektury
- *emulaci* - virtualizaci pro operační systémy nebo také simulace architektur
- *příkazové řádky* - využívající *REPL* (read-eval-print loop) pro interaktivní ovládání v textovém uživatelském prostředí

Prvními kompilery byly *assemblery* a pochází z 40. let minulého století. Ty však popisovaly pouze ruční metodu pro překládání do *strojového kódu*.
Naopak první interprety používali příkazové jazyky vysoké úrovně k ovládání počítačů a procesorů. Velmi známým příkladem je *Microsoft BASIC*, který vznikl v roce 1975. [6]

[6] wiki: Microsoft Basic
