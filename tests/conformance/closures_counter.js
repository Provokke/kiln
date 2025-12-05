function makeCounter() { let n = 0; return { inc() { n++; return n; }, get: () => n }; }
const a = makeCounter(), b = makeCounter();
a.inc(); a.inc(); b.inc();
console.log(a.get(), b.get());
function adder(x) { return y => z => x + y + z; }
console.log(adder(1)(2)(3));
