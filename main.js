const ctx = new AudioContext();
await ctx.audioWorklet.addModule('worklet.js');
const node = new AudioWorkletNode(ctx, 'source', {
  outputChannelCount: [2]
});
node.connect(ctx.destination);

let wasm = await fetch('./wasm/pkg/wasm_bg.wasm')
  .then(rsp => rsp.arrayBuffer());
node.port.postMessage({wasm});

if(document.readyState !== 'complete') {
  await new Promise(resolve =>
    document.addEventListener('DOMContentLoaded', resolve));
}

document.getElementById('play').addEventListener('input', ev => {
  if(ev.currentTarget.checked)
    ctx.resume();
  else
    ctx.suspend();
});

const size = 256;
let bufLeft = new Float32Array(size);
let bufRight = new Float32Array(size);
node.port.postMessage({buffer: {left: bufLeft, right: bufRight }});

node.port.onmessage = ev => {
  bufLeft.set(ev.data.buffer.left);
  bufRight.set(ev.data.buffer.right);
}

function drawFrame() {
  let dLeft = 'M ';
  for(let i = 0; i < size; i++) {
    dLeft += `${i * 6.28 / size} ${bufLeft[i]} L `;
  }
  dLeft += `6.283 ${bufLeft[0]}`;
  document.getElementById('pathLeft').setAttribute('d', dLeft);
  let dRight = 'M ';
  for(let i = 0; i < size; i++) {
    dRight += `${i * 6.28 / size} ${bufRight[i]} L `;
  }
  dRight += `6.283 ${bufRight[0]}`;
  document.getElementById('pathRight').setAttribute('d', dRight);
  requestAnimationFrame(drawFrame);
  node.port.postMessage({buffer: {left: bufLeft, right: bufRight }});
}

requestAnimationFrame(drawFrame);
