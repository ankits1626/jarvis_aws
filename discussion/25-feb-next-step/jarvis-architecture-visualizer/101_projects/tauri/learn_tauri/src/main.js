// In Tauri 2 with "withGlobalTauri": true,
// the API is available on window.__TAURI__
const { invoke } = window.__TAURI__.core;

let greetInputEl;
let greetMsgEl;
let addResultEl;
let sysInfoEl;
let counterEl;

async function greet() {
  const greeting = await invoke("greet", { name: greetInputEl.value });
  greetMsgEl.textContent = greeting;
}

async function add() {
  const sum = await invoke("add_numbers", { a: 10, b: 32 });
  addResultEl.textContent = `10 + 32 = ${sum}`;
}

async function getSystemInfo() {
  sysInfoEl.textContent = await invoke("system_info");
}

async function incrementCounter() {
  const count = await invoke("increment_counter");
  counterEl.textContent = `Count: ${count}`;
}

window.addEventListener("DOMContentLoaded", () => {
  greetInputEl = document.querySelector("#greet-input");
  greetMsgEl = document.querySelector("#greet-msg");
  addResultEl = document.querySelector("#add-result");

  document.querySelector("#greet-form").addEventListener("submit", (e) => {
    e.preventDefault();
    greet();
  });

  document.querySelector("#add-btn").addEventListener("click", add);

  sysInfoEl = document.querySelector("#sys-info");
  document.querySelector("#sys-info-btn").addEventListener("click", getSystemInfo);

  counterEl = document.querySelector("#counter");
  document.querySelector("#counter-btn").addEventListener("click", incrementCounter);

});
