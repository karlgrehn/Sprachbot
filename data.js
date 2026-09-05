// Vokabeldaten: Deutsch -> Zielsprache, gruppiert nach Kategorie.
const LANGUAGES = {
  en: { name: "Englisch", flag: "🇬🇧" },
  fr: { name: "Französisch", flag: "🇫🇷" },
  es: { name: "Spanisch", flag: "🇪🇸" },
  sv: { name: "Schwedisch", flag: "🇸🇪" },
  nl: { name: "Niederländisch", flag: "🇳🇱" },
  ar: { name: "Arabisch", flag: "🇸🇦" },
};

const CATEGORIES = {
  begruessung: "Begrüßung",
  zahlen: "Zahlen",
  farben: "Farben",
  alltag: "Alltag",
};

// Jeder Eintrag: [Deutsch, Zielsprache]
const VOCAB = {
  en: {
    begruessung: [["Hallo","Hello"],["Guten Morgen","Good morning"],["Guten Abend","Good evening"],["Auf Wiedersehen","Goodbye"],["Bitte","Please"],["Danke","Thank you"],["Ja","Yes"],["Nein","No"],["Entschuldigung","Sorry"],["Wie geht es dir?","How are you?"]],
    zahlen: [["eins","one"],["zwei","two"],["drei","three"],["vier","four"],["fünf","five"],["sechs","six"],["sieben","seven"],["acht","eight"],["neun","nine"],["zehn","ten"]],
    farben: [["rot","red"],["blau","blue"],["grün","green"],["gelb","yellow"],["schwarz","black"],["weiß","white"],["orange","orange"],["lila","purple"],["rosa","pink"],["grau","gray"]],
    alltag: [["Wasser","water"],["Brot","bread"],["Haus","house"],["Auto","car"],["Buch","book"],["Zeit","time"],["Freund","friend"],["Arbeit","work"],["Essen","food"],["Schule","school"]],
  },
  fr: {
    begruessung: [["Hallo","Salut"],["Guten Morgen","Bonjour"],["Guten Abend","Bonsoir"],["Auf Wiedersehen","Au revoir"],["Bitte","S'il te plaît"],["Danke","Merci"],["Ja","Oui"],["Nein","Non"],["Entschuldigung","Pardon"],["Wie geht es dir?","Comment ça va?"]],
    zahlen: [["eins","un"],["zwei","deux"],["drei","trois"],["vier","quatre"],["fünf","cinq"],["sechs","six"],["sieben","sept"],["acht","huit"],["neun","neuf"],["zehn","dix"]],
    farben: [["rot","rouge"],["blau","bleu"],["grün","vert"],["gelb","jaune"],["schwarz","noir"],["weiß","blanc"],["orange","orange"],["lila","violet"],["rosa","rose"],["grau","gris"]],
    alltag: [["Wasser","eau"],["Brot","pain"],["Haus","maison"],["Auto","voiture"],["Buch","livre"],["Zeit","temps"],["Freund","ami"],["Arbeit","travail"],["Essen","nourriture"],["Schule","école"]],
  },
  es: {
    begruessung: [["Hallo","Hola"],["Guten Morgen","Buenos días"],["Guten Abend","Buenas noches"],["Auf Wiedersehen","Adiós"],["Bitte","Por favor"],["Danke","Gracias"],["Ja","Sí"],["Nein","No"],["Entschuldigung","Perdón"],["Wie geht es dir?","¿Cómo estás?"]],
    zahlen: [["eins","uno"],["zwei","dos"],["drei","tres"],["vier","cuatro"],["fünf","cinco"],["sechs","seis"],["sieben","siete"],["acht","ocho"],["neun","nueve"],["zehn","diez"]],
    farben: [["rot","rojo"],["blau","azul"],["grün","verde"],["gelb","amarillo"],["schwarz","negro"],["weiß","blanco"],["orange","naranja"],["lila","morado"],["rosa","rosa"],["grau","gris"]],
    alltag: [["Wasser","agua"],["Brot","pan"],["Haus","casa"],["Auto","coche"],["Buch","libro"],["Zeit","tiempo"],["Freund","amigo"],["Arbeit","trabajo"],["Essen","comida"],["Schule","escuela"]],
  },
  sv: {
    begruessung: [["Hallo","Hej"],["Guten Morgen","God morgon"],["Guten Abend","God kväll"],["Auf Wiedersehen","Hej då"],["Bitte","Snälla"],["Danke","Tack"],["Ja","Ja"],["Nein","Nej"],["Entschuldigung","Förlåt"],["Wie geht es dir?","Hur mår du?"]],
    zahlen: [["eins","ett"],["zwei","två"],["drei","tre"],["vier","fyra"],["fünf","fem"],["sechs","sex"],["sieben","sju"],["acht","åtta"],["neun","nio"],["zehn","tio"]],
    farben: [["rot","röd"],["blau","blå"],["grün","grön"],["gelb","gul"],["schwarz","svart"],["weiß","vit"],["orange","orange"],["lila","lila"],["rosa","rosa"],["grau","grå"]],
    alltag: [["Wasser","vatten"],["Brot","bröd"],["Haus","hus"],["Auto","bil"],["Buch","bok"],["Zeit","tid"],["Freund","vän"],["Arbeit","arbete"],["Essen","mat"],["Schule","skola"]],
  },
  nl: {
    begruessung: [["Hallo","Hallo"],["Guten Morgen","Goedemorgen"],["Guten Abend","Goedenavond"],["Auf Wiedersehen","Tot ziens"],["Bitte","Alsjeblieft"],["Danke","Dank je"],["Ja","Ja"],["Nein","Nee"],["Entschuldigung","Sorry"],["Wie geht es dir?","Hoe gaat het?"]],
    zahlen: [["eins","een"],["zwei","twee"],["drei","drie"],["vier","vier"],["fünf","vijf"],["sechs","zes"],["sieben","zeven"],["acht","acht"],["neun","negen"],["zehn","tien"]],
    farben: [["rot","rood"],["blau","blauw"],["grün","groen"],["gelb","geel"],["schwarz","zwart"],["weiß","wit"],["orange","oranje"],["lila","paars"],["rosa","roze"],["grau","grijs"]],
    alltag: [["Wasser","water"],["Brot","brood"],["Haus","huis"],["Auto","auto"],["Buch","boek"],["Zeit","tijd"],["Freund","vriend"],["Arbeit","werk"],["Essen","eten"],["Schule","school"]],
  },
  ar: {
    begruessung: [["Hallo","مرحبا"],["Guten Morgen","صباح الخير"],["Guten Abend","مساء الخير"],["Auf Wiedersehen","مع السلامة"],["Bitte","من فضلك"],["Danke","شكرا"],["Ja","نعم"],["Nein","لا"],["Entschuldigung","آسف"],["Wie geht es dir?","كيف حالك؟"]],
    zahlen: [["eins","واحد"],["zwei","اثنان"],["drei","ثلاثة"],["vier","أربعة"],["fünf","خمسة"],["sechs","ستة"],["sieben","سبعة"],["acht","ثمانية"],["neun","تسعة"],["zehn","عشرة"]],
    farben: [["rot","أحمر"],["blau","أزرق"],["grün","أخضر"],["gelb","أصفر"],["schwarz","أسود"],["weiß","أبيض"],["orange","برتقالي"],["lila","بنفسجي"],["rosa","وردي"],["grau","رمادي"]],
    alltag: [["Wasser","ماء"],["Brot","خبز"],["Haus","بيت"],["Auto","سيارة"],["Buch","كتاب"],["Zeit","وقت"],["Freund","صديق"],["Arbeit","عمل"],["Essen","طعام"],["Schule","مدرسة"]],
  },
};
