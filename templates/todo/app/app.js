// To-Do starter — add/complete/delete + filters, persisted in localStorage.
'use strict';

const STORE_KEY = 'todo.items.v1';
let filter = 'all';

function load() {
  try {
    return JSON.parse(localStorage.getItem(STORE_KEY)) || [];
  } catch {
    return [];
  }
}
function save(items) {
  localStorage.setItem(STORE_KEY, JSON.stringify(items));
}

function visibleItems() {
  const items = load();
  if (filter === 'open') return items.filter((t) => !t.done);
  if (filter === 'done') return items.filter((t) => t.done);
  return items;
}

function render() {
  const list = document.getElementById('todo-list');
  list.innerHTML = '';
  for (const item of visibleItems()) {
    const li = document.createElement('li');
    li.className = item.done ? 'done' : '';
    li.innerHTML = `<input type="checkbox" ${item.done ? 'checked' : ''}/> <span></span> <button class="del" title="Delete">✕</button>`;
    li.querySelector('span').textContent = item.text;
    li.querySelector('input').addEventListener('change', () => {
      const items = load();
      const t = items.find((x) => x.id === item.id);
      t.done = !t.done;
      save(t && items);
      render();
    });
    li.querySelector('.del').addEventListener('click', () => {
      save(load().filter((x) => x.id !== item.id));
      render();
    });
    list.appendChild(li);
  }
  const open = load().filter((t) => !t.done).length;
  document.getElementById('count').textContent = `${open} open`;
}

document.getElementById('add-form').addEventListener('submit', (e) => {
  e.preventDefault();
  const input = document.getElementById('todo-input');
  const text = input.value.trim();
  if (!text) return;
  const items = load();
  items.push({ id: crypto.randomUUID(), text, done: false });
  save(items);
  input.value = '';
  render();
});

for (const btn of document.querySelectorAll('.filters button')) {
  btn.addEventListener('click', () => {
    filter = btn.dataset.filter;
    for (const b of document.querySelectorAll('.filters button')) b.classList.toggle('active', b === btn);
    render();
  });
}

render();
