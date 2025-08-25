import * as wasm from './wasm/pkg/wasm.js';

class RandomNoiseProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.port.onmessage = ev => {
      if(ev.data.wasm) {
        wasm.initSync({ module: ev.data.wasm })
        this.processFn = wasm.process;
        this.handle = wasm.Instance.new_handle(sampleRate);
      } else if(this.handle && ev.data.buffers) {
        let buffers = ev.data.buffers;
        wasm.get_sample(buffers.left, buffers.right, this.handle);
        this.port.postMessage({buffers}, [buffers.left.buffer, buffers.right.buffer]);
      }
    }
  }

  process(inputs, outputs, parameters) {
    const output = outputs[0];
    if(this.handle) {
      this.processFn(output[0], output[1], this.handle);
    }
    return true;
  }
}

registerProcessor('source', RandomNoiseProcessor);
