// Minimalistische Sprachlern-App: Multiple-Choice-Quiz, Fortschritt in localStorage.
const STORAGE_KEY = "sprachbot_progress";

const state = {
  lang: null,
  category: null,
  queue: [],
  index: 0,
  correct: 0,
  locked: false,
};

function loadProgress(){
  try{ return JSON.parse(localStorage.getItem(STORAGE_KEY)) || {}; }
  catch{ return {}; }
}
function saveProgress(p){
  localStorage.setItem(STORAGE_KEY, JSON.stringify(p));
}
function addScore(lang, n){
  const p = loadProgress();
  p[lang] = (p[lang] || 0) + n;
  saveProgress(p);
  renderStats();
}
function totalScore(){
  const p = loadProgress();
  return Object.values(p).reduce((a,b)=>a+b,0);
}

function show(viewId){
  document.querySelectorAll(".view").forEach(v => v.classList.add("hidden"));
  document.getElementById(viewId).classList.remove("hidden");
}

function renderStats(){
  document.getElementById("stats").textContent = `${totalScore()} gelernt`;
}

function renderHome(){
  const grid = document.getElementById("langGrid");
  const p = loadProgress();
  grid.innerHTML = "";
  Object.entries(LANGUAGES).forEach(([code, info]) => {
    const div = document.createElement("div");
    div.className = "tile";
    div.innerHTML = `<span class="flag">${info.flag}</span><span class="name">${info.name}</span><span class="sub">${p[code] || 0} gelernt</span>`;
    div.addEventListener("click", () => openCategories(code));
    grid.appendChild(div);
  });
  show("view-home");
}

function openCategories(langCode){
  state.lang = langCode;
  document.getElementById("catSubtitle").textContent = `${LANGUAGES[langCode].name} — Kategorie wählen`;
  const grid = document.getElementById("catGrid");
  grid.innerHTML = "";
  Object.entries(CATEGORIES).forEach(([catCode, catName]) => {
    const words = VOCAB[langCode][catCode];
    const div = document.createElement("div");
    div.className = "tile";
    div.innerHTML = `<span class="name">${catName}</span><span class="sub">${words.length} Wörter</span>`;
    div.addEventListener("click", () => startQuiz(langCode, catCode));
    grid.appendChild(div);
  });
  show("view-categories");
}

function shuffle(arr){
  const a = arr.slice();
  for(let i = a.length - 1; i > 0; i--){
    const j = Math.floor(Math.random() * (i + 1));
    [a[i], a[j]] = [a[j], a[i]];
  }
  return a;
}

function startQuiz(langCode, catCode){
  state.lang = langCode;
  state.category = catCode;
  state.queue = shuffle(VOCAB[langCode][catCode]);
  state.index = 0;
  state.correct = 0;
  renderQuestion();
  show("view-quiz");
}

function renderQuestion(){
  state.locked = false;
  const total = state.queue.length;
  document.getElementById("quizBar").style.width = `${(state.index / total) * 100}%`;
  document.getElementById("quizFeedback").textContent = "";

  const [deWord, targetWord] = state.queue[state.index];
  document.getElementById("quizWord").textContent = deWord;

  const pool = VOCAB[state.lang][state.category];
  const distractors = shuffle(pool.filter(([de]) => de !== deWord)).slice(0, 3).map(([, t]) => t);
  const options = shuffle([targetWord, ...distractors]);

  const optionsEl = document.getElementById("quizOptions");
  optionsEl.innerHTML = "";
  options.forEach(opt => {
    const btn = document.createElement("button");
    btn.className = "opt";
    btn.textContent = opt;
    btn.addEventListener("click", () => selectAnswer(btn, opt, targetWord));
    optionsEl.appendChild(btn);
  });
}

function selectAnswer(btn, chosen, correctAnswer){
  if(state.locked) return;
  state.locked = true;
  const isCorrect = chosen === correctAnswer;
  btn.classList.add(isCorrect ? "correct" : "wrong");
  if(isCorrect) state.correct++;
  else {
    [...document.querySelectorAll(".opt")].forEach(o => {
      if(o.textContent === correctAnswer) o.classList.add("correct");
    });
  }
  document.getElementById("quizFeedback").textContent = isCorrect ? "Richtig!" : `Richtig wäre: ${correctAnswer}`;

  setTimeout(() => {
    state.index++;
    if(state.index >= state.queue.length){
      finishQuiz();
    } else {
      renderQuestion();
    }
  }, 900);
}

function finishQuiz(){
  document.getElementById("quizBar").style.width = "100%";
  addScore(state.lang, state.correct);
  document.getElementById("doneScore").textContent =
    `${state.correct} von ${state.queue.length} richtig`;
  show("view-done");
}

document.addEventListener("click", e => {
  const back = e.target.dataset && e.target.dataset.back;
  if(back === "home") renderHome();
  if(back === "categories") openCategories(state.lang);
});

document.getElementById("btnRetry").addEventListener("click", () => startQuiz(state.lang, state.category));
document.getElementById("btnHome").addEventListener("click", renderHome);

renderStats();
renderHome();
