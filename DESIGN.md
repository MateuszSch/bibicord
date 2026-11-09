# BibiCord – design system

Kierunek: znajomy układ komunikatora dla graczy (szyna kanałów po lewej,
scena po prawej), wykonany ciszej i bardziej minimalistycznie. Interfejs
ma znikać w trakcie rozmowy; kolor pojawia się tylko w stanach: mówienie,
na żywo, akcja główna, błąd.

Źródło prawdy dla wartości: `src/ui/theme.rs`. Ten plik opisuje zasady.

## Kolor

Chłodne neutralne, lekko niebieskie (odcień grafitowo-stalowy). Powierzchnie
rosną jasnością w stronę użytkownika. Akcent to neonowa zieleń, wyraźnie inna
niż fioletowo-niebieski Discorda; nie kopiujemy wartości z palet innych
produktów. Akcent jest jasny i nasycony, więc używaj go oszczędnie.
`success` (`#2dc770`) to spokojniejsza, chłodniejsza zieleń: nie zamieniaj ich.

| Token | Hex | Użycie |
|---|---|---|
| `bg_stage` | `#0a0c0f` | scena wideo, tło pod streamem |
| `bg_base` | `#101216` | tło okna, ekran startowy |
| `bg_sidebar` | `#15181d` | panel boczny, panele użytkownika |
| `bg_content` | `#1b1e24` | czat, karty, modale |
| `bg_raised` | `#22262c` | pola tekstowe, najechany wiersz |
| `bg_selected` | `#2a2e36` | zaznaczony wiersz, przycisk drugorzędny |
| `border` | `#2a2e36` | separatory 1px |
| `border_strong` | `#3b404a` | krawędzie pól w hover, pasek przewijania |
| `text` | `#eef1f5` | tekst główny |
| `text_secondary` | `#aeb5bf` | nazwy kanałów, opisy |
| `text_muted` | `#99a1ad` | metadane, czas, nagłówki sekcji, placeholdery (5.8:1 na `bg_raised`) |
| `brand` | `#5cff3a` / hover `#4ae62c` | akcja główna, zaznaczenie, link, fokus (12.6:1 na `bg_content`) |
| `on_brand` | `#0a0c0f` | tekst i ikony na pełnym tle `brand` (14.8:1); biały na neonowej zieleni jest nieczytelny |
| `success` | `#2dc770` | „połączono”, P2P, „Skopiowano” |
| `speaking` | `#2ee6ff` | mówienie: pierścień awatara, ramka kafelka |
| `danger` | `#ed4b55` / hover `#d83c46` | NA ŻYWO, rozłącz, wyciszony, błędy |
| `warning` | `#f2a93b` | relay (połączenie pośrednie) |

Awatary: 8 nasyconych barw przypisywanych deterministycznie z ID węzła.

## Typografia

Jedna rodzina: **Inter** (OFL, osadzona w binarce). Skala ~1.15.

| Rola | Rozmiar | Krój |
|---|---|---|
| display | 26 | Bold |
| title | 18 | SemiBold |
| heading | 15 | SemiBold |
| body | 15 | Regular |
| label | 14 | Medium |
| small | 13 | Regular/Medium |
| caption | 12 | Medium |
| section | 11 | Bold, wersaliki, +0.6px rozstrzelenia |

Ikony: **Phosphor** (regular; fill dla stanów aktywnych), jedna grubość
kreski. Bez emoji i znaków Unicode zamiast ikon.

## Odstępy, promienie, głębia

- Siatka 4px: 2, 4, 6, 8, 12, 16, 20, 24, 32, 48.
- Promienie: 4 (znaczniki), 6 (wiersze listy), 8 (przyciski, pola), 12 (kafelki, karty, modale), pełny (awatary, okrągłe przyciski sceny).
- Głębia deklarowana raz: modale i menu mają cień (0, 8) rozmycie 24, bez obramowania; powierzchnie płaskie oddziela różnica tonu.

## Komponenty

- **Wiersz kanału / uczestnika**: wysokość 32/30, promień 6, hover `bg_raised`, zaznaczony `bg_selected` + tekst `text`.
- **Awatar**: koło z inicjałem na barwie użytkownika; mówienie = pierścień `speaking` 2px z odstępem; kropka statusu w rogu.
- **Znacznik NA ŻYWO**: `danger`, wersaliki 10/Bold, promień 4.
- **Przyciski**: primary (`brand`), secondary (`bg_selected`), danger, ghost; wysokość 36 (34 w panelach), promień 8; ikonowe 32×32 promień 6.
- **Kafelek uczestnika**: 16:9, promień 12, tło z barwy użytkownika przyciemnione, ramka `speaking` 2px przy mówieniu.
- **Pole tekstowe**: `bg_raised`, bez obramowania, fokus = obrys `brand` 1px.
- **Modal**: `bg_content`, promień 12, stopka `bg_sidebar` z akcjami po prawej.

## Ruch

Tylko zmiana stanu: przejścia kolorów hover/aktywne 120 ms, wejście modala
140 ms. Pierścień mówienia zanika 300 ms po ciszy. Brak animacji ozdobnych.

## Ton

Polski, krótko, czasowniki na przyciskach („Dołącz do głosu”, „Rozpocznij stream”).
Błędy mówią, co się stało i co zrobić.
