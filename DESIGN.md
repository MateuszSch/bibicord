# BibiCord – design system

Kierunek: znajomy układ komunikatora dla graczy (szyna kanałów po lewej,
scena po prawej), wykonany ciszej i bardziej minimalistycznie. Interfejs
ma znikać w trakcie rozmowy; kolor pojawia się tylko w stanach: mówienie,
na żywo, akcja główna, błąd.

Źródło prawdy dla wartości: `src/ui/theme.rs`. Ten plik opisuje zasady.

## Kolor

Chłodne neutralne, lekko niebieskie. Powierzchnie rosną jasnością
w stronę użytkownika.

| Token | Hex | Użycie |
|---|---|---|
| `bg_stage` | `#0b0c0e` | scena wideo, tło pod streamem |
| `bg_base` | `#111214` | tło okna, ekran startowy |
| `bg_sidebar` | `#16171a` | panel boczny, panele użytkownika |
| `bg_content` | `#1c1d21` | czat, karty, modale |
| `bg_raised` | `#232428` | pola tekstowe, najechany wiersz |
| `bg_selected` | `#2b2d32` | zaznaczony wiersz, przycisk drugorzędny |
| `border` | `#2b2d32` | separatory 1px |
| `border_strong` | `#3a3c42` | krawędzie pól w hover, pasek przewijania |
| `text` | `#f2f3f5` | tekst główny |
| `text_secondary` | `#b5bac1` | nazwy kanałów, opisy |
| `text_muted` | `#949ba4` | metadane, czas, nagłówki sekcji, placeholdery (5.5:1 na `bg_raised`) |
| `brand` | `#5b6cff` / hover `#4c5cf0` / active `#4050d8` | akcja główna, zaznaczenie, link, fokus |
| `success` | `#2dc770` | mówienie, „połączono”, P2P |
| `danger` | `#ed4b55` / hover `#d83c46` | NA ŻYWO, rozłącz, wyciszony, błędy |
| `warning` | `#f0b232` | relay (połączenie pośrednie) |

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
- **Awatar**: koło z inicjałem na barwie użytkownika; mówienie = pierścień `success` 2px z odstępem; kropka statusu w rogu.
- **Znacznik NA ŻYWO**: `danger`, wersaliki 10/Bold, promień 4.
- **Przyciski**: primary (`brand`), secondary (`bg_selected`), danger, ghost; wysokość 36 (34 w panelach), promień 8; ikonowe 32×32 promień 6.
- **Kafelek uczestnika**: 16:9, promień 12, tło z barwy użytkownika przyciemnione, ramka `success` 2px przy mówieniu.
- **Pole tekstowe**: `bg_raised`, bez obramowania, fokus = obrys `brand` 1px.
- **Modal**: `bg_content`, promień 12, stopka `bg_sidebar` z akcjami po prawej.

## Ruch

Tylko zmiana stanu: przejścia kolorów hover/aktywne 120 ms, wejście modala
140 ms. Pierścień mówienia zanika 300 ms po ciszy. Brak animacji ozdobnych.

## Ton

Polski, krótko, czasowniki na przyciskach („Dołącz do głosu”, „Rozpocznij stream”).
Błędy mówią, co się stało i co zrobić.
