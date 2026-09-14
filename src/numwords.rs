//! Multilingual number words, measurement units, currencies and English
//! contractions used by the prose encoder.

/// Value of a number word: `Some((value, is_multiplier))`.
/// Multipliers (hundred, thousand, million…) multiply the running total.
pub fn number_word(lang: &str, w: &str) -> Option<(f64, bool)> {
    let common = |w: &str| -> Option<(f64, bool)> {
        Some(match w {
            "hundred" | "sto" | "stu" | "hundert" | "cent" | "cien" | "ciento" | "cento"
            | "cem" | "сто" | "honderd" | "hundra" | "hundrede" | "hundre" | "sata" | "yüz"
            | "száz" | "sută" => (100.0, true),
            "thousand" | "tysiąc" | "tysiące" | "tysięcy" | "tausend" | "mille" | "mil"
            | "mila" | "тысяча" | "тысячи" | "тысяч" | "тисяча" | "тисячі" | "тисяч" | "tisíc"
            | "tisíce" | "duizend" | "tusen" | "tusind" | "tuhat" | "tuhatta" | "bin" | "ezer"
            | "mie" | "mii" => (1e3, true),
            "million" | "millions" | "milion" | "miliony" | "milionów" | "millionen" | "millón"
            | "millones" | "milione" | "milioni" | "milhão" | "milhões" | "миллион"
            | "миллиона" | "миллионов" | "мільйон" | "мільйони" | "мільйонів" | "milionů"
            | "miljoen" | "miljoner" | "millioner" | "miljoona" | "miljoonaa" | "milyon"
            | "millió" | "milioane" => (1e6, true),
            "billion"
            | "billions"
            | "miliard"
            | "miliardy"
            | "miliardów"
            | "milliarde"
            | "milliarden"
            | "milliards"
            | "miliardo"
            | "miliardi"
            | "bilhão"
            | "bilhões"
            | "миллиард"
            | "миллиарда"
            | "миллиардов"
            | "мільярд"
            | "мільярди"
            | "мільярдів"
            | "miliarda"
            | "miljard"
            | "miljarder"
            | "milliarder"
            | "miljardi"
            | "miljardia"
            | "milyar"
            | "milliárd"
            | "miliarde" => (1e9, true),
            "trillion"
            | "trillions"
            | "bilion"
            | "biliony"
            | "bilionów"
            | "billionen"
            | "billón"
            | "billones"
            | "bilione"
            | "bilioni"
            | "trilhão"
            | "trilhões"
            | "триллион"
            | "триллиона"
            | "триллионов"
            | "трильйон"
            | "трильйони"
            | "трильйонів"
            | "biljoen"
            | "biljon"
            | "trilyon"
            | "billió"
            | "trilion" => (1e12, true),
            _ => return None,
        })
    };
    if let Some(x) = common(w) {
        return Some(x);
    }
    let v = match lang {
        "en" => match w {
            "one" => 1.0,
            "two" => 2.0,
            "three" => 3.0,
            "four" => 4.0,
            "five" => 5.0,
            "six" => 6.0,
            "seven" => 7.0,
            "eight" => 8.0,
            "nine" => 9.0,
            "ten" => 10.0,
            "eleven" => 11.0,
            "twelve" => 12.0,
            "dozen" => 12.0,
            "thirteen" => 13.0,
            "fourteen" => 14.0,
            "fifteen" => 15.0,
            "sixteen" => 16.0,
            "seventeen" => 17.0,
            "eighteen" => 18.0,
            "nineteen" => 19.0,
            "twenty" => 20.0,
            "thirty" => 30.0,
            "forty" => 40.0,
            "fifty" => 50.0,
            "sixty" => 60.0,
            "seventy" => 70.0,
            "eighty" => 80.0,
            "ninety" => 90.0,
            _ => return None,
        },
        "pl" => match w {
            "jeden" | "jedna" | "jedno" | "jednego" | "jednej" => 1.0,
            "dwa" | "dwie" | "dwóch" | "dwoje" | "dwu" => 2.0,
            "trzy" | "trzech" | "troje" => 3.0,
            "cztery" | "czterech" | "czworo" => 4.0,
            "pięć" | "pięciu" | "pięcioro" => 5.0,
            "sześć" | "sześciu" => 6.0,
            "siedem" | "siedmiu" => 7.0,
            "osiem" | "ośmiu" => 8.0,
            "dziewięć" | "dziewięciu" => 9.0,
            "dziesięć" | "dziesięciu" => 10.0,
            "jedenaście" | "jedenastu" => 11.0,
            "dwanaście" | "dwunastu" => 12.0,
            "trzynaście" | "trzynastu" => 13.0,
            "czternaście" | "czternastu" => 14.0,
            "piętnaście" | "piętnastu" => 15.0,
            "szesnaście" | "szesnastu" => 16.0,
            "siedemnaście" | "siedemnastu" => 17.0,
            "osiemnaście" | "osiemnastu" => 18.0,
            "dziewiętnaście" | "dziewiętnastu" => 19.0,
            "dwadzieścia" | "dwudziestu" => 20.0,
            "trzydzieści" | "trzydziestu" => 30.0,
            "czterdzieści" | "czterdziestu" => 40.0,
            "pięćdziesiąt" | "pięćdziesięciu" => 50.0,
            "sześćdziesiąt" | "sześćdziesięciu" => 60.0,
            "siedemdziesiąt" | "siedemdziesięciu" => 70.0,
            "osiemdziesiąt" | "osiemdziesięciu" => 80.0,
            "dziewięćdziesiąt" | "dziewięćdziesięciu" => 90.0,
            "dwieście" | "dwustu" => 200.0,
            "trzysta" | "trzystu" => 300.0,
            "czterysta" | "czterystu" => 400.0,
            "pięćset" | "pięciuset" => 500.0,
            "sześćset" => 600.0,
            "siedemset" => 700.0,
            "osiemset" => 800.0,
            "dziewięćset" => 900.0,
            _ => return None,
        },
        "de" => match w {
            "eins" => 1.0,
            "zwei" => 2.0,
            "drei" => 3.0,
            "vier" => 4.0,
            "fünf" => 5.0,
            "sechs" => 6.0,
            "sieben" => 7.0,
            "acht" => 8.0,
            "neun" => 9.0,
            "zehn" => 10.0,
            "elf" => 11.0,
            "zwölf" => 12.0,
            "dreizehn" => 13.0,
            "vierzehn" => 14.0,
            "fünfzehn" => 15.0,
            "sechzehn" => 16.0,
            "siebzehn" => 17.0,
            "achtzehn" => 18.0,
            "neunzehn" => 19.0,
            "zwanzig" => 20.0,
            "dreißig" | "dreissig" => 30.0,
            "vierzig" => 40.0,
            "fünfzig" => 50.0,
            "sechzig" => 60.0,
            "siebzig" => 70.0,
            "achtzig" => 80.0,
            "neunzig" => 90.0,
            _ => return None,
        },
        "fr" => match w {
            "deux" => 2.0,
            "trois" => 3.0,
            "quatre" => 4.0,
            "cinq" => 5.0,
            "six" => 6.0,
            "sept" => 7.0,
            "huit" => 8.0,
            "neuf" => 9.0,
            "dix" => 10.0,
            "onze" => 11.0,
            "douze" => 12.0,
            "treize" => 13.0,
            "quatorze" => 14.0,
            "quinze" => 15.0,
            "seize" => 16.0,
            "vingt" => 20.0,
            "trente" => 30.0,
            "quarante" => 40.0,
            "cinquante" => 50.0,
            "soixante" => 60.0,
            "cents" => 100.0,
            _ => return None,
        },
        "es" => match w {
            "dos" => 2.0,
            "tres" => 3.0,
            "cuatro" => 4.0,
            "cinco" => 5.0,
            "seis" => 6.0,
            "siete" => 7.0,
            "ocho" => 8.0,
            "nueve" => 9.0,
            "diez" => 10.0,
            "once" => 11.0,
            "doce" => 12.0,
            "trece" => 13.0,
            "catorce" => 14.0,
            "quince" => 15.0,
            "veinte" => 20.0,
            "treinta" => 30.0,
            "cuarenta" => 40.0,
            "cincuenta" => 50.0,
            "sesenta" => 60.0,
            "setenta" => 70.0,
            "ochenta" => 80.0,
            "noventa" => 90.0,
            "doscientos" => 200.0,
            "trescientos" => 300.0,
            "cuatrocientos" => 400.0,
            "quinientos" => 500.0,
            _ => return None,
        },
        "it" => match w {
            "due" => 2.0,
            "tre" => 3.0,
            "quattro" => 4.0,
            "cinque" => 5.0,
            "sei" => 6.0,
            "sette" => 7.0,
            "otto" => 8.0,
            "nove" => 9.0,
            "dieci" => 10.0,
            "undici" => 11.0,
            "dodici" => 12.0,
            "tredici" => 13.0,
            "quattordici" => 14.0,
            "quindici" => 15.0,
            "sedici" => 16.0,
            "venti" => 20.0,
            "trenta" => 30.0,
            "quaranta" => 40.0,
            "cinquanta" => 50.0,
            "sessanta" => 60.0,
            "settanta" => 70.0,
            "ottanta" => 80.0,
            "novanta" => 90.0,
            _ => return None,
        },
        "pt" => match w {
            "dois" | "duas" => 2.0,
            "três" | "tres" => 3.0,
            "quatro" => 4.0,
            "cinco" => 5.0,
            "seis" => 6.0,
            "sete" => 7.0,
            "oito" => 8.0,
            "nove" => 9.0,
            "dez" => 10.0,
            "onze" => 11.0,
            "doze" => 12.0,
            "treze" => 13.0,
            "catorze" | "quatorze" => 14.0,
            "quinze" => 15.0,
            "vinte" => 20.0,
            "trinta" => 30.0,
            "quarenta" => 40.0,
            "cinquenta" => 50.0,
            "sessenta" => 60.0,
            "setenta" => 70.0,
            "oitenta" => 80.0,
            "noventa" => 90.0,
            "duzentos" => 200.0,
            "trezentos" => 300.0,
            "quinhentos" => 500.0,
            _ => return None,
        },
        "ru" => match w {
            "два" | "две" | "двух" => 2.0,
            "три" | "трёх" | "трех" => 3.0,
            "четыре" | "четырёх" | "четырех" => 4.0,
            "пять" | "пяти" => 5.0,
            "шесть" | "шести" => 6.0,
            "семь" | "семи" => 7.0,
            "восемь" | "восьми" => 8.0,
            "девять" | "девяти" => 9.0,
            "десять" | "десяти" => 10.0,
            "одиннадцать" => 11.0,
            "двенадцать" => 12.0,
            "тринадцать" => 13.0,
            "четырнадцать" => 14.0,
            "пятнадцать" => 15.0,
            "шестнадцать" => 16.0,
            "семнадцать" => 17.0,
            "восемнадцать" => 18.0,
            "девятнадцать" => 19.0,
            "двадцать" | "двадцати" => 20.0,
            "тридцать" | "тридцати" => 30.0,
            "сорок" | "сорока" => 40.0,
            "пятьдесят" | "пятидесяти" => 50.0,
            "шестьдесят" => 60.0,
            "семьдесят" => 70.0,
            "восемьдесят" => 80.0,
            "девяносто" => 90.0,
            "двести" => 200.0,
            "триста" => 300.0,
            "четыреста" => 400.0,
            "пятьсот" => 500.0,
            _ => return None,
        },
        "uk" => match w {
            "два" | "дві" | "двох" => 2.0,
            "три" | "трьох" => 3.0,
            "чотири" | "чотирьох" => 4.0,
            "п'ять" | "п'яти" => 5.0,
            "шість" | "шести" => 6.0,
            "сім" | "семи" => 7.0,
            "вісім" | "восьми" => 8.0,
            "дев'ять" | "дев'яти" => 9.0,
            "десять" | "десяти" => 10.0,
            "одинадцять" => 11.0,
            "дванадцять" => 12.0,
            "тринадцять" => 13.0,
            "чотирнадцять" => 14.0,
            "п'ятнадцять" => 15.0,
            "шістнадцять" => 16.0,
            "сімнадцять" => 17.0,
            "вісімнадцять" => 18.0,
            "дев'ятнадцять" => 19.0,
            "двадцять" => 20.0,
            "тридцять" => 30.0,
            "сорок" => 40.0,
            "п'ятдесят" => 50.0,
            "шістдесят" => 60.0,
            "сімдесят" => 70.0,
            "вісімдесят" => 80.0,
            "дев'яносто" => 90.0,
            "двісті" => 200.0,
            "триста" => 300.0,
            "чотириста" => 400.0,
            "п'ятсот" => 500.0,
            _ => return None,
        },
        "cs" | "sk" => match w {
            "dva" | "dvě" | "dve" | "dvou" => 2.0,
            "tři" | "tri" | "tří" => 3.0,
            "čtyři" | "štyri" => 4.0,
            "pět" | "päť" => 5.0,
            "šest" | "šesť" => 6.0,
            "sedm" | "sedem" => 7.0,
            "osm" | "osem" => 8.0,
            "devět" | "deväť" => 9.0,
            "deset" | "desať" => 10.0,
            "jedenáct" | "jedenásť" => 11.0,
            "dvanáct" | "dvanásť" => 12.0,
            "třináct" | "trinásť" => 13.0,
            "čtrnáct" | "štrnásť" => 14.0,
            "patnáct" | "pätnásť" => 15.0,
            "dvacet" | "dvadsať" => 20.0,
            "třicet" | "tridsať" => 30.0,
            "čtyřicet" | "štyridsať" => 40.0,
            "padesát" | "päťdesiat" => 50.0,
            "šedesát" | "šesťdesiat" => 60.0,
            "sedmdesát" | "sedemdesiat" => 70.0,
            "osmdesát" | "osemdesiat" => 80.0,
            "devadesát" | "deväťdesiat" => 90.0,
            "dvěstě" | "dvě stě" | "dvesto" => 200.0,
            "třista" | "tristo" => 300.0,
            "pětset" | "päťsto" => 500.0,
            _ => return None,
        },
        "nl" => match w {
            "twee" => 2.0,
            "drie" => 3.0,
            "vier" => 4.0,
            "vijf" => 5.0,
            "zes" => 6.0,
            "zeven" => 7.0,
            "acht" => 8.0,
            "negen" => 9.0,
            "tien" => 10.0,
            "elf" => 11.0,
            "twaalf" => 12.0,
            "dertien" => 13.0,
            "veertien" => 14.0,
            "vijftien" => 15.0,
            "twintig" => 20.0,
            "dertig" => 30.0,
            "veertig" => 40.0,
            "vijftig" => 50.0,
            "zestig" => 60.0,
            "zeventig" => 70.0,
            "tachtig" => 80.0,
            "negentig" => 90.0,
            _ => return None,
        },
        _ => return None,
    };
    Some((v, false))
}

/// Percent words (attach `%` to the preceding number).
pub fn is_percent_word(w: &str) -> bool {
    matches!(
        w,
        "percent"
            | "percents"
            | "pct"
            | "procent"
            | "procenta"
            | "procentów"
            | "procenty"
            | "procentu"
            | "proc"
            | "prozent"
            | "pourcent"
            | "porcentaje"
            | "процент"
            | "процента"
            | "процентов"
            | "відсоток"
            | "відсотки"
            | "відсотків"
            | "процентів"
            | "prosent"
            | "prosenttia"
            | "prosentti"
            | "yüzde"
            | "százalék"
            | "százaléka"
            | "százalékos"
    )
}

/// Two-word percent phrases ("per cent", "por ciento", "la sută").
pub fn is_percent_phrase(a: &str, b: &str) -> bool {
    matches!(
        (a, b),
        ("per", "cent")
            | ("per", "cento")
            | ("por", "ciento")
            | ("por", "cento")
            | ("la", "sută")
            | ("pour", "cent")
    )
}

/// Unit word → compact suffix appended directly to the number (`5h`, `30min`, `2km`).
pub fn unit_suffix(w: &str) -> Option<&'static str> {
    Some(match w {
        "hours" | "hour" | "hrs" | "hr" | "godzin" | "godziny" | "godzina" | "godzinę" | "godz"
        | "stunden" | "stunde" | "heures" | "heure" | "horas" | "hora" | "ore" | "ora" | "uur"
        | "uren" | "часов" | "часа" | "час" | "годин" | "години" | "година" | "hodin"
        | "hodiny" | "hodina" | "hodín" | "timmar" | "timme" | "timer" | "tuntia" | "tunti"
        | "saat" | "óra" | "órát" | "ore." => "h",
        "minutes" | "minute" | "mins" | "min" | "minut" | "minuty" | "minuta" | "minutę"
        | "minuten" | "minutos" | "minuto" | "minuti" | "минут" | "минуты" | "минута"
        | "минуту" | "хвилин" | "хвилини" | "хвилина" | "хвилину" | "minút" | "minuter"
        | "minutter" | "minuuttia" | "minuutti" | "dakika" | "perc" | "percet" | "minute." => "min",
        "seconds" | "second" | "secs" | "sec" | "sekund" | "sekundy" | "sekunda" | "sekundę"
        | "sekunden" | "sekunde" | "secondes" | "seconde" | "segundos" | "segundo" | "secondi"
        | "secondo" | "seconden" | "секунд" | "секунды" | "секунда" | "секунду" | "секунди"
        | "sekúnd" | "sekunder" | "sekuntia" | "sekunti" | "saniye" | "másodperc" | "secunde"
        | "secundă" => "s",
        "milliseconds"
        | "millisecond"
        | "msec"
        | "milisekund"
        | "milisekundy"
        | "millisekunden"
        | "millisecondes"
        | "milisegundos"
        | "millisecondi"
        | "миллисекунд"
        | "мілісекунд" => "ms",
        "days" | "day" | "dni" | "dzień" | "dniach" | "tage" | "tag" | "tagen" | "jours"
        | "jour" | "días" | "día" | "dias" | "dia" | "giorni" | "giorno" | "dagen" | "dag"
        | "дней" | "дня" | "день" | "днів" | "дні" | "dnů" | "dny" | "den" | "dní" | "dagar"
        | "dage" | "päivää" | "päivä" | "gün" | "nap" | "napot" | "zile" | "zi" => "d",
        "weeks" | "week" | "wks" | "wk" | "tygodni" | "tygodnie" | "tydzień" | "tygodniu"
        | "wochen" | "woche" | "semaines" | "semaine" | "semanas" | "semana" | "settimane"
        | "settimana" | "weken" | "недель" | "недели" | "неделя" | "неделю" | "тижнів"
        | "тижні" | "тиждень" | "týdnů" | "týdny" | "týden" | "týždňov" | "veckor" | "vecka"
        | "uger" | "uge" | "uker" | "uke" | "viikkoa" | "viikko" | "hafta" | "hét" | "hetet"
        | "săptămâni" | "săptămână" => "wk",
        "months" | "month" | "mos" | "miesięcy" | "miesiące" | "miesiąc" | "miesiącu"
        | "monate" | "monat" | "monaten" | "mois" | "meses" | "mes" | "mês" | "mesi" | "mese"
        | "maanden" | "maand" | "месяцев" | "месяца" | "месяц" | "місяців" | "місяці"
        | "місяць" | "měsíců" | "měsíce" | "měsíc" | "mesiacov" | "månader" | "månad"
        | "måneder" | "måned" | "kuukautta" | "kuukausi" | "ay" | "hónap" | "hónapot" | "luni"
        | "lună" => "mo",
        "years" | "year" | "yrs" | "yr" | "lat" | "lata" | "rok" | "roku" | "latach" | "jahre"
        | "jahr" | "jahren" | "ans" | "années" | "année" | "años" | "año" | "anos" | "ano"
        | "anni" | "anno" | "jaar" | "jaren" | "лет" | "года" | "год" | "років" | "роки"
        | "рік" | "let" | "roky" | "rokov" | "år" | "vuotta" | "vuosi" | "yıl" | "év" | "évet"
        | "ani" => "yr",
        "kilometers"
        | "kilometres"
        | "kilometer"
        | "kms"
        | "kilometrów"
        | "kilometry"
        | "kilometr"
        | "kilomètres"
        | "kilomètre"
        | "kilómetros"
        | "kilómetro"
        | "quilômetros"
        | "quilómetros"
        | "chilometri"
        | "chilometro"
        | "километров"
        | "километра"
        | "километр"
        | "кілометрів"
        | "кілометри"
        | "кілометр"
        | "kilometrů"
        | "kilometrov" => "km",
        "meters" | "metres" | "meter" | "metre" | "metrów" | "metry" | "metr" | "mètres"
        | "mètre" | "metros" | "metro" | "metri" | "метров" | "метра" | "метр" | "метрів"
        | "метри" | "metrů" | "metrov" => "m",
        "centimeters"
        | "centimetres"
        | "centimeter"
        | "centymetrów"
        | "centymetry"
        | "zentimeter"
        | "centimètres"
        | "centímetros"
        | "centimetri"
        | "сантиметров"
        | "сантиметрів" => "cm",
        "millimeters"
        | "millimetres"
        | "millimeter"
        | "milimetrów"
        | "millimètres"
        | "milímetros"
        | "millimetri"
        | "миллиметров"
        | "міліметрів" => "mm",
        "kilograms"
        | "kilogram"
        | "kilos"
        | "kilo"
        | "kilogramów"
        | "kilogramy"
        | "kilogramm"
        | "kilogrammes"
        | "kilogramme"
        | "kilogramos"
        | "kilogramo"
        | "chilogrammi"
        | "chilogrammo"
        | "quilogramas"
        | "quilos"
        | "килограммов"
        | "килограмма"
        | "килограмм"
        | "кілограмів"
        | "кілограми"
        | "kilogramů" => "kg",
        "grams" | "gram" | "gramów" | "gramy" | "gramm" | "grammes" | "gramme" | "gramos"
        | "gramo" | "grammi" | "grammo" | "граммов" | "грамма" | "грамм" | "грамів" | "gramů" => {
            "g"
        }
        "tons" | "tonnes" | "ton" | "tonne" | "tonnen" | "toneladas" | "tonnellate" | "тонн"
        | "тонны" | "тонна" | "тонн." => "t",
        "liters" | "litres" | "liter" | "litre" | "litrów" | "litry" | "litr" | "litros"
        | "litri" | "литров" | "литра" | "літрів" | "litrů" => "l",
        "degrees" | "degree" | "stopni" | "stopnie" | "grad" | "degrés" | "grados" | "gradi"
        | "graus" | "graden" | "градусов" | "градуса" | "градусів" | "stupňů" | "stupně" => {
            "°"
        }
        "bytes" | "byte" | "bajtów" | "bajty" | "bajt" | "octets" | "octet" => "B",
        "kilobytes" | "kilobyte" | "kilobajtów" => "KB",
        "megabytes" | "megabyte" | "megabajtów" | "megabajty" | "megaoctets" => "MB",
        "gigabytes" | "gigabyte" | "gigabajtów" | "gigabajty" | "gigaoctets" | "gigs" | "gig" => {
            "GB"
        }
        "terabytes" | "terabyte" | "terabajtów" | "teraoctets" => "TB",
        "times" | "razy" | "raz" | "mal" | "fois" | "veces" | "vez" | "volte" | "volta"
        | "vezes" | "keer" | "maal" | "раз" | "раза" | "разів" | "рази" | "krát" | "gånger"
        | "gange" | "ganger" | "kertaa" | "kez" | "kere" | "defa" | "alkalommal" | "ori" => "×",
        "people" | "persons" | "person" | "osób" | "osoby" | "osoba" | "personen" | "personnes"
        | "personne" | "personas" | "persona" | "persone" | "pessoas" | "pessoa" | "человек"
        | "людей" | "осіб" | "людей." | "lidí" | "osob" | "personer" | "henkilöä" | "ihmistä"
        | "kişi" | "ember" | "fő" | "persoane" => "👤",
        "users"
        | "user"
        | "użytkowników"
        | "użytkownicy"
        | "użytkownik"
        | "benutzer"
        | "nutzer"
        | "utilisateurs"
        | "utilisateur"
        | "usuarios"
        | "usuario"
        | "utenti"
        | "utente"
        | "usuários"
        | "usuário"
        | "gebruikers"
        | "gebruiker"
        | "пользователей"
        | "пользователи"
        | "пользователь"
        | "користувачів"
        | "користувачі"
        | "користувач"
        | "uživatelů"
        | "uživatelé"
        | "uživatel" => "👤",
        "employees"
        | "employee"
        | "pracowników"
        | "pracownicy"
        | "pracownik"
        | "mitarbeiter"
        | "employés"
        | "employé"
        | "empleados"
        | "empleado"
        | "dipendenti"
        | "dipendente"
        | "funcionários"
        | "funcionário"
        | "сотрудников"
        | "сотрудники"
        | "працівників"
        | "працівники"
        | "zaměstnanců"
        | "zaměstnanci" => "👤",
        "items" | "item" | "elementów" | "elementy" | "element" | "elemente" | "éléments"
        | "élément" | "elementos" | "elemento" | "elementi" | "элементов" | "элемента"
        | "елементів" | "položek" | "položky" => "×item",
        "pieces" | "piece" | "pcs" | "sztuk" | "sztuki" | "sztuka" | "szt" | "stück" | "pièces"
        | "pièce" | "piezas" | "pieza" | "pezzi" | "pezzo" | "peças" | "peça" | "stuks"
        | "штук" | "штуки" | "штука" | "шт" | "kusů" | "kusy" | "kus" => "pcs",
        _ => return None,
    })
}

/// Currency word → (symbol, is_prefix).
pub fn currency(w: &str) -> Option<(&'static str, bool)> {
    Some(match w {
        "dollars" | "dollar" | "usd" | "dolarów" | "dolary" | "dolar" | "dólares" | "dólar"
        | "dollari" | "dollaro" | "долларов" | "доллара" | "доллар" | "доларів" | "долари"
        | "долар" | "dolarů" | "bucks" | "buck" => ("$", true),
        "euros" | "euro" | "eur" | "евро" | "євро" | "eurów" | "eura" | "eurech" | "eur." => {
            ("€", true)
        }
        "pounds" | "pound" | "gbp" | "quid" | "funtów" | "funty" | "funt" | "pfund" | "livres"
        | "libras" | "libra" | "sterline" | "sterlina" | "фунтов" | "фунта" | "фунт" | "фунтів"
        | "liber" => ("£", true),
        "yen" | "jpy" | "jenów" | "jeny" | "jen" | "иен" | "єн" | "jenů" => ("¥", true),
        "złotych" | "złote" | "złoty" | "złotego" | "zł" | "pln" | "zloty" | "zlotys"
        | "złotówek" | "złotówki" => ("zł", false),
        "koron" | "korony" | "korona" | "kronen" | "krone" | "kronor" | "krona" | "kroner"
        | "korun" | "koruny" | "koruna" | "kč" | "czk" | "sek" | "nok" | "dkk" => ("kr", false),
        "rubles" | "ruble" | "roubles" | "rouble" | "rub" | "рублей" | "рубля" | "рубль"
        | "руб" | "rubli" | "rublů" => ("₽", false),
        "hryvnias" | "hryvnia" | "uah" | "гривень" | "гривні" | "гривня" | "грн" | "hrywien"
        | "hrywny" | "hrywna" => ("₴", false),
        "francs" | "franc" | "chf" | "franków" | "franki" | "franken" => ("CHF", false),
        "rupees" | "rupee" | "inr" | "rupii" | "rupie" => ("₹", true),
        "yuan" | "cny" | "rmb" | "juanów" | "juany" | "juan" | "renminbi" => ("¥", true),
        "reais" | "real" | "brl" => ("R$", true),
        "forint" | "forintów" | "forinty" | "huf" | "forintot" => ("Ft", false),
        "lei" | "leu" | "ron" => ("lei", false),
        "lira" | "liras" | "lirası" | "lir" => ("₺", true),
        _ => return None,
    })
}

/// English contractions → replacement (may be empty = drop).
pub fn contraction(w: &str) -> Option<&'static str> {
    Some(match w {
        "don't" | "doesn't" | "didn't" | "isn't" | "aren't" | "wasn't" | "weren't" | "haven't"
        | "hasn't" | "hadn't" | "wouldn't" | "ain't" => "¬",
        "won't" => "¬将",
        "can't" | "cannot" | "couldn't" => "¬可",
        "shouldn't" => "¬应",
        "mustn't" | "needn't" => "¬必",
        "i'm" | "i've" | "i'll" | "i'd" => "I",
        "you're" | "you've" | "you'll" | "you'd" => "you",
        "we're" | "we've" | "we'll" | "we'd" => "we",
        "they're" | "they've" | "they'll" | "they'd" => "they",
        "he's" | "he'll" | "he'd" => "he",
        "she's" | "she'll" | "she'd" => "she",
        "it's" | "it'll" | "it'd" | "that's" | "there's" | "there're" | "there'll" | "here's" => "",
        "what's" => "what",
        "who's" => "who",
        "where's" => "where",
        "how's" => "how",
        "let's" => "let's",
        "y'all" => "you",
        "o'clock" => "h",
        _ => return None,
    })
}

/// Ordinal words → compact ordinal.
pub fn ordinal(lang: &str, w: &str) -> Option<&'static str> {
    Some(match (lang, w) {
        ("en", "first") => "1st",
        ("en", "second") => "2nd",
        ("en", "third") => "3rd",
        ("en", "fourth") => "4th",
        ("en", "fifth") => "5th",
        ("en", "sixth") => "6th",
        ("en", "seventh") => "7th",
        ("en", "eighth") => "8th",
        ("en", "ninth") => "9th",
        ("en", "tenth") => "10th",
        ("en", "twentieth") => "20th",
        ("en", "hundredth") => "100th",
        (
            "pl",
            "pierwszy" | "pierwsza" | "pierwsze" | "pierwszego" | "pierwszej" | "pierwszym"
            | "pierwszą",
        ) => "1.",
        ("pl", "drugi" | "druga" | "drugie" | "drugiego" | "drugiej" | "drugim" | "drugą") => "2.",
        (
            "pl",
            "trzeci" | "trzecia" | "trzecie" | "trzeciego" | "trzeciej" | "trzecim" | "trzecią",
        ) => "3.",
        (
            "pl",
            "czwarty" | "czwarta" | "czwarte" | "czwartego" | "czwartej" | "czwartym" | "czwartą",
        ) => "4.",
        ("pl", "piąty" | "piąta" | "piąte" | "piątego" | "piątej" | "piątym" | "piątą") => {
            "5."
        }
        ("de", "erste" | "ersten" | "erster" | "erstes" | "erstem") => "1.",
        ("de", "zweite" | "zweiten" | "zweiter" | "zweites" | "zweitem") => "2.",
        ("de", "dritte" | "dritten" | "dritter" | "drittes" | "drittem") => "3.",
        ("de", "vierte" | "vierten" | "vierter" | "viertes") => "4.",
        ("de", "fünfte" | "fünften" | "fünfter" | "fünftes") => "5.",
        ("fr", "premier" | "première" | "premiers" | "premières") => "1er",
        ("fr", "deuxième" | "deuxièmes") => "2e",
        ("fr", "troisième" | "troisièmes") => "3e",
        ("fr", "quatrième") => "4e",
        ("fr", "cinquième") => "5e",
        ("es", "primero" | "primera" | "primer" | "primeros" | "primeras") => "1º",
        ("es", "segundo" | "segunda" | "segundos" | "segundas") => "2º",
        ("es", "tercero" | "tercera" | "tercer" | "terceros" | "terceras") => "3º",
        ("es", "cuarto" | "cuarta") => "4º",
        ("es", "quinto" | "quinta") => "5º",
        ("it", "primo" | "prima" | "primi" | "prime") => "1º",
        ("it", "secondo" | "seconda" | "secondi" | "seconde") => "2º",
        ("it", "terzo" | "terza" | "terzi" | "terze") => "3º",
        ("pt", "primeiro" | "primeira" | "primeiros" | "primeiras") => "1º",
        ("pt", "segundo" | "segunda" | "segundos" | "segundas") => "2º",
        ("pt", "terceiro" | "terceira" | "terceiros" | "terceiras") => "3º",
        (
            "ru",
            "первый" | "первая" | "первое" | "первые" | "первого" | "первой" | "первом" | "первую",
        ) => "1-й",
        (
            "ru",
            "второй" | "вторая" | "второе" | "вторые" | "второго" | "второй." | "втором" | "вторую",
        ) => "2-й",
        (
            "ru",
            "третий" | "третья" | "третье" | "третьи" | "третьего" | "третьей" | "третьем"
            | "третью",
        ) => "3-й",
        (
            "uk",
            "перший" | "перша" | "перше" | "перші" | "першого" | "першої" | "першому" | "першу",
        ) => "1-й",
        (
            "uk",
            "другий" | "друга" | "друге" | "другі" | "другого" | "другої" | "другому" | "другу",
        ) => "2-й",
        (
            "uk",
            "третій" | "третя" | "третє" | "треті" | "третього" | "третьої" | "третьому" | "третю",
        ) => "3-й",
        ("cs", "první" | "prvního" | "prvním" | "prvních") => "1.",
        ("cs", "druhý" | "druhá" | "druhé" | "druhého" | "druhém" | "druhou") => "2.",
        ("cs", "třetí" | "třetího" | "třetím") => "3.",
        _ => return None,
    })
}

/// Format a number value compactly (`30`, `1.5`, `30k`, `3M`, `2B`).
pub fn format_value(v: f64) -> String {
    fn trim(x: f64) -> String {
        if (x - x.round()).abs() < 1e-9 {
            format!("{}", x.round() as i64)
        } else {
            let s = format!("{x:.2}");
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        }
    }
    if v >= 1e12 && (v / 1e12 * 100.0).round() / 100.0 == v / 1e12 {
        format!("{}T", trim(v / 1e12))
    } else if v >= 1e9 && (v / 1e9 * 100.0).round() / 100.0 == v / 1e9 {
        format!("{}B", trim(v / 1e9))
    } else if v >= 1e6 && (v / 1e6 * 100.0).round() / 100.0 == v / 1e6 {
        format!("{}M", trim(v / 1e6))
    } else if v >= 1e4 && v % 1e3 == 0.0 {
        format!("{}k", trim(v / 1e3))
    } else {
        trim(v)
    }
}
