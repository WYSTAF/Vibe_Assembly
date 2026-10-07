// Planner starter — goals with localStorage persistence + a static week grid.
// Deliberately minimal so the modes have room to grow it into YOUR planner.
'use strict';

const STORE_KEY = 'planner.goals.v1';

function loadGoals() {
  try {
    return JSON.parse(localStorage.getItem(STORE_KEY)) || [];
  } catch {
    return [];
  }
}
function saveGoals(goals) {
  localStorage.setItem(STORE_KEY, JSON.stringify(goals));
}

function renderGoals() {
  const list = document.getElementById('goal-list');
  list.innerHTML = '';
  for (const [i, goal] of loadGoals().entries()) {
    const li = document.createElement('li');
    li.className = goal.done ? 'done' : '';
    li.innerHTML = `<input type="checkbox" ${goal.done ? 'checked' : ''} /> <span></span>`;
    li.querySelector('span').textContent = goal.text;
    li.querySelector('input').addEventListener('change', () => {
      const goals = loadGoals();
      goals[i].done = !goals[i].done;
      saveGoals(goals);
      renderGoals();
    });
    list.appendChild(li);
  }
}

document.getElementById('goal-form').addEventListener('submit', (e) => {
  e.preventDefault();
  const input = document.getElementById('goal-input');
  const text = input.value.trim();
  if (!text) return;
  const goals = loadGoals();
  goals.push({ text, done: false, created: new Date().toISOString() });
  saveGoals(goals);
  input.value = '';
  renderGoals();
});

// Static week grid — a natural first ticket: "make days clickable".
const weekGrid = document.getElementById('week-grid');
const DAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
for (const day of DAYS) {
  const div = document.createElement('div');
  div.className = 'day';
  div.innerHTML = `<b>${day}</b>—`;
  weekGrid.appendChild(div);
}

renderGoals();
