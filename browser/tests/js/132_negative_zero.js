console.log(-0);
console.log(String(-0), `${-0}`, -0 + '', (-0).toString(), JSON.stringify(-0));
console.log([-0], { z: -0 });
console.log(Object.is(-0, 0), -0 === 0, 1 / -0, Math.sign(-0));
console.log(Math.round(-0.4), Math.ceil(-0.5), Math.trunc(-0.9), -0 * 5);
console.log((-0).toFixed(2), Math.abs(-0), Math.min(0, -0), Math.max(-0, 0));
console.log([-0].includes(0), [-0].indexOf(0), new Set([0, -0]).size);
console.log(JSON.parse('-0'), Object.is(JSON.parse('-0'), -0), parseInt('-0'), parseFloat('-0'));
