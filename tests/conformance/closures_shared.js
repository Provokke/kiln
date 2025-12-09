function pair() { let v = 0; return [() => ++v, () => v]; }
const pp = pair(); const inc = pp[0], get = pp[1];
inc(); inc(); inc();
console.log(get());
function outer() { let x = 1; function mid() { function inner() { return x++; } return inner; } return mid(); }
const f = outer(); f(); console.log(f());
