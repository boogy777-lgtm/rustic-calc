//! Coordinate mode labels, regional names and domain-error presentation.
use super::Lang;
use crate::coordinates::{CoordinateField, Error};

pub struct Text {
    pub title: &'static str,
    pub region: &'static str,
    pub template: &'static str,
    pub custom: &'static str,
    pub definition: &'static str,
    pub reset: &'static str,
    pub info: &'static str,
    pub back: &'static str,
    pub latitude: &'static str,
    pub longitude: &'static str,
    pub height: &'static str,
    pub northing: &'static str,
    pub easting: &'static str,
    pub source: &'static str,
    pub revision: &'static str,
    pub original: &'static str,
    pub modified: &'static str,
    pub custom_status: &'static str,
    pub no_templates: &'static str,
    pub accuracy: &'static str,
    pub session: &'static str,
    pub help: &'static str,
}

pub fn text(lang: Lang) -> &'static Text {
    match lang {
        Lang::Ru => &RU,
        Lang::En => &EN,
    }
}

static RU: Text = Text {
    title: "WGS84 → МСК",
    region: "Регион",
    template: "Система и зона",
    custom: "Свои параметры (PROJ)",
    definition: "Параметры PROJ — можно редактировать",
    reset: "Сбросить шаблон",
    info: "Справка / лицензия (F2)",
    back: "Назад (Esc / F2)",
    latitude: "Широта WGS84, десятичные градусы",
    longitude: "Долгота WGS84, десятичные градусы",
    height: "Эллипсоидальная высота h, м",
    northing: "X — север, м (Northing)",
    easting: "Y — восток, м (Easting)",
    source: "Источник",
    revision: "Ревизия",
    original: "исходный открытый шаблон",
    modified: "ИЗМЕНЁННЫЕ параметры — проверьте",
    custom_status: "Свои параметры — проверьте систему и датум.",
    no_templates: "Открытого шаблона нет — введите свои параметры PROJ.",
    accuracy: "Шаблоны не аттестованы. Точность проверяйте по опорным точкам; регион и зону выбирайте вручную.",
    session: "Правки сохраняются только в текущем сеансе. Высота h — WGS84, не высота над уровнем моря.",
    help: "WGS84 → МСК\r\n\r\n1. Выберите регион и нужную систему/зону. Номер зоны нельзя надёжно определить только по региону.\r\n2. Шаблон заполняет редактируемое поле PROJ. Можно вставить свои параметры. Правки каждого шаблона и пользовательского поля сохраняются в памяти до закрытия программы; переключение режима/языка/DPI их не теряет. Сброс удаляет правку только выбранного шаблона.\r\n3. Введите широту, долготу в десятичных градусах и эллипсоидальную высоту WGS84 в метрах. Допускается точка или запятая. Если высота неизвестна, явное h=0 — допущение, влияющее на пересчёт. Пустые и нечисловые значения не заменяются нулями.\r\n4. Enter вычисляет, Tab/Shift+Tab перемещают фокус. Результаты доступны для выделения и копирования. Ctrl+G открывает этот режим. F2/Esc возвращают из справки.\r\n\r\nПОДДЕРЖИВАЕМЫЙ ФОРМАТ\r\n+proj=tmerc +lat_0=0 +lon_0=38.48333333333 +k=1 +x_0=2250000 +y_0=-5712900.566 +ellps=krass +towgs84=23.57,-140.95,-79.8,0,0.35,0.79,-0.22 +units=m +no_defs\r\n\r\nЭтот пример — МСК-50, зона 2. Все параметры открыты для редактирования. Числа PROJ используют десятичную точку. Поддерживаются k/k_0, ellps=krass/bessel/WGS84/GRS80 либо a и rf, towgs84 из 3 или 7 чисел, no_defs, type=crs. Для проекции самого WGS84 допустим datum=WGS84 вместо towgs84. WKT, EPSG-коды, другие проекции, сетки nadgrids, произвольные оси и неизвестные параметры отклоняются, а не игнорируются.\r\n\r\nСОГЛАШЕНИЯ И ОГРАНИЧЕНИЯ\r\nВход: широта, долгота, h WGS84. Выход: X=Northing, Y=Easting, метры. В PROJ x_0 — восточное смещение, y_0 — северное: не перепутайте с геодезическими X/Y. towgs84 задаёт локальный датум → WGS84 (position-vector): dx/dy/dz в метрах, rx/ry/rz в угловых секундах, масштаб в ppm. Для направления WGS84 → МСК применяется обратное малоугловое преобразование. Параметры MapInfo нельзя вставлять напрямую: соглашения о знаках поворотов могут отличаться.\r\n\r\nРеализована поперечная Меркатора по ряду Крюгера шестого порядка. Расстояние по долготе от осевого меридиана ограничено ±30°; это не проверка кадастровых границ. Нет автоматического выбора зоны, геоидных сеток, эпох/скоростей движения плит и преобразования высот.\r\n\r\nКаталог содержит 262 определения для 79 регионов; для остальных предусмотрен ручной ввод без выдуманных чисел. Открытые определения могут быть устаревшими или ошибочными. Совпадение с математическим эталоном не означает геодезической точности. Для межевания и юридически значимых работ необходимы официальные параметры и проверка по независимым опорным точкам. Три знака после запятой — формат результата, не обещание миллиметровой точности.\r\n\r\nИСТОЧНИК И ЛИЦЕНЗИЯ КАТАЛОГА\r\n",
};

static EN: Text = Text {
    title: "WGS84 → MSK",
    region: "Region",
    template: "Coordinate system and zone",
    custom: "Custom parameters (PROJ)",
    definition: "PROJ parameters — editable",
    reset: "Reset template",
    info: "Help / license (F2)",
    back: "Back (Esc / F2)",
    latitude: "WGS84 latitude, decimal degrees",
    longitude: "WGS84 longitude, decimal degrees",
    height: "Ellipsoidal height h, metres",
    northing: "X — Northing, metres",
    easting: "Y — Easting, metres",
    source: "Source",
    revision: "Revision",
    original: "original community template",
    modified: "MODIFIED parameters — verify",
    custom_status: "Custom parameters — verify the coordinate system and datum.",
    no_templates: "No open template available — enter your own PROJ parameters.",
    accuracy: "Templates are not certified. Verify against control points; select the region and zone manually.",
    session: "Edits last for this session only. Height h is WGS84 ellipsoidal height, not height above sea level.",
    help: "WGS84 → MSK\r\n\r\n1. Select the region, coordinate system and zone. A region alone cannot reliably determine the zone.\r\n2. The template fills an editable PROJ field. Paste a custom definition or edit a template. Each template/custom draft is retained in memory until the application closes, including across mode/language/DPI changes. Reset affects only the current draft.\r\n3. Enter latitude and longitude in decimal degrees, and WGS84 ellipsoidal height in metres. Dot/comma decimal separators are accepted. An explicit h=0 for unknown height is an assumption that affects the transformation. Blank/invalid values never silently become zero.\r\n4. Enter calculates; Tab/Shift+Tab move focus. Results are selectable/copyable. Ctrl+G opens this mode. F2/Esc return from this help.\r\n\r\nSUPPORTED FORMAT\r\n+proj=tmerc +lat_0=0 +lon_0=38.48333333333 +k=1 +x_0=2250000 +y_0=-5712900.566 +ellps=krass +towgs84=23.57,-140.95,-79.8,0,0.35,0.79,-0.22 +units=m +no_defs\r\n\r\nThis example is MSK-50 zone 2. All parameters are editable. PROJ numbers require a decimal dot. Supported: k/k_0, ellps=krass/bessel/WGS84/GRS80 or a with rf, 3/7-value towgs84, no_defs, type=crs. A WGS84 projection may use datum=WGS84 instead of towgs84. WKT, EPSG codes, other projections, grid shifts, arbitrary axes and unknown parameters are rejected, never ignored.\r\n\r\nCONVENTIONS AND LIMITS\r\nInput: latitude, longitude, WGS84 h. Output: X=Northing, Y=Easting, metres. In PROJ, x_0 is false easting and y_0 is false northing: do not confuse these with surveying X/Y. towgs84 specifies local datum → WGS84 (position-vector), translations in metres, rotations in arcseconds, scale in ppm. WGS84 → MSK uses its legacy small-angle inverse. Do not paste MapInfo parameters directly: rotation sign conventions can differ.\r\n\r\nTransverse Mercator uses the sixth-order Krueger series. Longitude is limited to ±30° from the central meridian; this does not check administrative/cadastral boundaries. No automatic zone selection, geoid grids, coordinate epochs/plate velocities or vertical datum transformation.\r\n\r\nThe catalogue has 262 definitions covering 79 regions. Other regions allow manual definitions, not invented numbers. Community definitions may be obsolete or wrong. Numerical reference agreement does not establish surveying accuracy. Legally significant/survey work requires official parameters and independent control points. Three decimal places are output formatting, not millimetre-accuracy certification.\r\n\r\nCATALOGUE SOURCE AND LICENSE\r\n",
};

pub fn error(lang: Lang, error: &Error) -> String {
    let ru = lang == Lang::Ru;
    match error {
        Error::EmptyDefinition => if ru { "Выберите шаблон или введите параметры PROJ." } else { "Select a template or enter PROJ parameters." }.into(),
        Error::Missing(key) => format!("{}: {key}", if ru { "Не задан параметр" } else { "Missing parameter" }),
        Error::Invalid(key) => format!("{}: {key}", if ru { "Неверный параметр" } else { "Invalid parameter" }),
        Error::Unsupported(key) => format!("{}: {key}", if ru { "Не поддерживается" } else { "Unsupported parameter" }),
        Error::Duplicate(key) => format!("{}: {key}", if ru { "Повтор/конфликт параметров" } else { "Duplicate/conflicting parameter" }),
        Error::DatumRequired => if ru { "Нужен явный towgs84 (3/7 чисел) или datum=WGS84." } else { "Explicit towgs84 (3/7 values) or datum=WGS84 is required." }.into(),
        Error::Coordinate(field) => match (ru, field) {
            (true, CoordinateField::Latitude) => "Широта: число в десятичных градусах, −90 < φ < 90.",
            (true, CoordinateField::Longitude) => "Долгота: число в десятичных градусах от −180 до 180.",
            (true, CoordinateField::Height) => "Высота h: число от −10000 до 100000 м; неизвестная высота — явное допущение h=0.",
            (false, CoordinateField::Latitude) => "Latitude: decimal degrees, −90 < latitude < 90.",
            (false, CoordinateField::Longitude) => "Longitude: decimal degrees from −180 to 180.",
            (false, CoordinateField::Height) => "Height h: −10000 to 100000 m; unknown height requires an explicit h=0 assumption.",
        }.into(),
        Error::OutsideProjection => if ru { "Точка вне области расчёта (±30° от осевого меридиана). Проверьте зону и параметры." } else { "Point outside the calculation domain (±30° from central meridian). Check zone/parameters." }.into(),
    }
}

pub fn template_name(lang: Lang, name: &str) -> String {
    if lang == Lang::Ru {
        return name.into();
    }
    name.replace("Московская СК (МГГТ)", "Moscow (MGGT)")
        .replace("МСК г. Красноярска", "Krasnoyarsk local CRS")
        .replace("Санкт-Петербург", "Saint Petersburg")
        .replace("На базе СК-95", "based on SK-95")
        .replace("от СК-63", "from SK-63")
        .replace("от СК-95", "from SK-95")
        .replace("МСК", "MSK")
        .replace("Зона", "zone")
}

pub struct Region {
    pub id: &'static str,
    ru: &'static str,
    en: &'static str,
}

impl Region {
    pub fn name(&self, lang: Lang) -> &'static str {
        if lang == Lang::Ru {
            self.ru
        } else {
            self.en
        }
    }
}

macro_rules! regions {
    ($(($id:literal, $ru:literal, $en:literal)),+ $(,)?) => {
        pub static REGIONS: &[Region] = &[$(Region { id: $id, ru: $ru, en: $en }),+];
    };
}

// Regional groups, not inferred CRS identifiers or a statement about borders.
// Empty groups intentionally require a user-supplied definition.
regions![
    ("01", "Республика Адыгея", "Adygea"),
    ("02", "Республика Башкортостан", "Bashkortostan"),
    ("03", "Республика Бурятия", "Buryatia"),
    ("04", "Республика Алтай", "Altai Republic"),
    ("05", "Республика Дагестан", "Dagestan"),
    ("06", "Республика Ингушетия", "Ingushetia"),
    (
        "07",
        "Кабардино-Балкарская Республика",
        "Kabardino-Balkaria"
    ),
    ("08", "Республика Калмыкия", "Kalmykia"),
    (
        "09",
        "Карачаево-Черкесская Республика",
        "Karachay-Cherkessia"
    ),
    ("10", "Республика Карелия", "Karelia"),
    ("11", "Республика Коми", "Komi"),
    ("12", "Республика Марий Эл", "Mari El"),
    ("13", "Республика Мордовия", "Mordovia"),
    ("14", "Республика Саха (Якутия)", "Sakha (Yakutia)"),
    (
        "15",
        "Республика Северная Осетия — Алания",
        "North Ossetia — Alania"
    ),
    ("16", "Республика Татарстан", "Tatarstan"),
    ("17", "Республика Тыва", "Tuva"),
    ("18", "Удмуртская Республика", "Udmurtia"),
    ("19", "Республика Хакасия", "Khakassia"),
    ("20", "Чеченская Республика", "Chechnya"),
    ("21", "Чувашская Республика", "Chuvashia"),
    ("22", "Алтайский край", "Altai Krai"),
    ("23", "Краснодарский край", "Krasnodar Krai"),
    ("24", "Красноярский край", "Krasnoyarsk Krai"),
    ("25", "Приморский край", "Primorsky Krai"),
    ("26", "Ставропольский край", "Stavropol Krai"),
    ("27", "Хабаровский край", "Khabarovsk Krai"),
    ("28", "Амурская область", "Amur Oblast"),
    ("29", "Архангельская область", "Arkhangelsk Oblast"),
    ("30", "Астраханская область", "Astrakhan Oblast"),
    ("31", "Белгородская область", "Belgorod Oblast"),
    ("32", "Брянская область", "Bryansk Oblast"),
    ("33", "Владимирская область", "Vladimir Oblast"),
    ("34", "Волгоградская область", "Volgograd Oblast"),
    ("35", "Вологодская область", "Vologda Oblast"),
    ("36", "Воронежская область", "Voronezh Oblast"),
    ("37", "Ивановская область", "Ivanovo Oblast"),
    ("38", "Иркутская область", "Irkutsk Oblast"),
    ("39", "Калининградская область", "Kaliningrad Oblast"),
    ("40", "Калужская область", "Kaluga Oblast"),
    ("41", "Камчатский край", "Kamchatka Krai"),
    (
        "42",
        "Кемеровская область — Кузбасс",
        "Kemerovo Oblast — Kuzbass"
    ),
    ("43", "Кировская область", "Kirov Oblast"),
    ("44", "Костромская область", "Kostroma Oblast"),
    ("45", "Курганская область", "Kurgan Oblast"),
    ("46", "Курская область", "Kursk Oblast"),
    ("47", "Ленинградская область", "Leningrad Oblast"),
    ("48", "Липецкая область", "Lipetsk Oblast"),
    ("49", "Магаданская область", "Magadan Oblast"),
    ("50", "Московская область", "Moscow Oblast"),
    ("51", "Мурманская область", "Murmansk Oblast"),
    ("52", "Нижегородская область", "Nizhny Novgorod Oblast"),
    ("53", "Новгородская область", "Novgorod Oblast"),
    ("54", "Новосибирская область", "Novosibirsk Oblast"),
    ("55", "Омская область", "Omsk Oblast"),
    ("56", "Оренбургская область", "Orenburg Oblast"),
    ("57", "Орловская область", "Oryol Oblast"),
    ("58", "Пензенская область", "Penza Oblast"),
    ("59", "Пермский край", "Perm Krai"),
    ("60", "Псковская область", "Pskov Oblast"),
    ("61", "Ростовская область", "Rostov Oblast"),
    ("62", "Рязанская область", "Ryazan Oblast"),
    ("63", "Самарская область", "Samara Oblast"),
    ("64", "Саратовская область", "Saratov Oblast"),
    ("65", "Сахалинская область", "Sakhalin Oblast"),
    ("66", "Свердловская область", "Sverdlovsk Oblast"),
    ("67", "Смоленская область", "Smolensk Oblast"),
    ("68", "Тамбовская область", "Tambov Oblast"),
    ("69", "Тверская область", "Tver Oblast"),
    ("70", "Томская область", "Tomsk Oblast"),
    ("71", "Тульская область", "Tula Oblast"),
    ("72", "Тюменская область", "Tyumen Oblast"),
    ("73", "Ульяновская область", "Ulyanovsk Oblast"),
    ("74", "Челябинская область", "Chelyabinsk Oblast"),
    ("75", "Забайкальский край", "Zabaykalsky Krai"),
    ("76", "Ярославская область", "Yaroslavl Oblast"),
    ("77", "Москва", "Moscow"),
    ("78", "Санкт-Петербург", "Saint Petersburg"),
    (
        "79",
        "Еврейская автономная область",
        "Jewish Autonomous Oblast"
    ),
    ("83", "Ненецкий автономный округ", "Nenets Autonomous Okrug"),
    (
        "86",
        "Ханты-Мансийский автономный округ — Югра",
        "Khanty-Mansi Autonomous Okrug — Yugra"
    ),
    (
        "87",
        "Чукотский автономный округ",
        "Chukotka Autonomous Okrug"
    ),
    (
        "89",
        "Ямало-Ненецкий автономный округ",
        "Yamalo-Nenets Autonomous Okrug"
    ),
    ("crimea", "Крым", "Crimea"),
    ("sevastopol", "Севастополь", "Sevastopol"),
    ("donetsk", "Донецк / область", "Donetsk / region"),
    ("luhansk", "Луганск / область", "Luhansk / region"),
    ("zaporizhzhia", "Запорожская область", "Zaporizhzhia Oblast"),
    ("kherson", "Херсонская область", "Kherson Oblast"),
];
