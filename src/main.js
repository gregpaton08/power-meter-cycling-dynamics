const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let powerEl, cadenceEl;
let lSmooth, lTorque, rSmooth, rTorque;
let lCtx, rCtx;

window.addEventListener("DOMContentLoaded", () => {
  powerEl = document.querySelector("#power-val");
  cadenceEl = document.querySelector("#cadence-val");
  lSmooth = document.querySelector("#l-smooth");
  lTorque = document.querySelector("#l-torque");
  rSmooth = document.querySelector("#r-smooth");
  rTorque = document.querySelector("#r-torque");

  lCtx = document.querySelector("#left-phase").getContext("2d");
  rCtx = document.querySelector("#right-phase").getContext("2d");

  document.querySelector("#connect-btn").addEventListener("click", async () => {
    const port = document.querySelector("#port-input").value;
    const status = document.querySelector("#status");
    status.innerText = await invoke("start_listening", { port });
  });

  // Listen for Rust events
  listen("cycling-data", (event) => {
    const data = event.payload;
    updateUI(data);
  });
});

function updateUI(data) {
  powerEl.innerText = data.instant_power;
  cadenceEl.innerText = data.cadence;
  
  lSmooth.innerText = data.l_pedal_smooth;
  lTorque.innerText = data.l_torque_eff;
  rSmooth.innerText = data.r_pedal_smooth;
  rTorque.innerText = data.r_torque_eff;

  drawPhase(lCtx, data.l_phase_start, data.l_phase_end, "Left");
  drawPhase(rCtx, data.r_phase_start, data.r_phase_end, "Right");
}

function drawPhase(ctx, startDeg, endDeg, label) {
  const w = ctx.canvas.width;
  const h = ctx.canvas.height;
  const cx = w / 2;
  const cy = h / 2;
  const r = w / 2 - 10;

  // Clear
  ctx.clearRect(0, 0, w, h);

  // Draw Background Circle (Crank)
  ctx.beginPath();
  ctx.arc(cx, cy, r, 0, 2 * Math.PI);
  ctx.strokeStyle = "#444";
  ctx.lineWidth = 2;
  ctx.stroke();

  // Draw Phase Arc
  // Convert degrees to radians. 0 deg is usually top (12 o'clock)
  // Canvas 0 is 3 o'clock. We offset by -90 deg (-PI/2)
  const startRad = (startDeg - 90) * (Math.PI / 180);
  const endRad = (endDeg - 90) * (Math.PI / 180);

  ctx.beginPath();
  ctx.arc(cx, cy, r, startRad, endRad);
  ctx.strokeStyle = "#00d2ff"; // Cyan for power phase
  ctx.lineWidth = 12;
  ctx.lineCap = "round";
  ctx.stroke();

  // Label
  ctx.fillStyle = "#fff";
  ctx.font = "14px Arial";
  ctx.textAlign = "center";
  ctx.fillText(label, cx, cy + 5);
}