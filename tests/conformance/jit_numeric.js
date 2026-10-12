function fib(n) { if (n < 2) return n; return fib(n - 1) + fib(n - 2); }
console.log(fib(22));
function collatz(n) { let steps = 0; while (n !== 1) { if (n % 2 === 0) n = n / 2; else n = 3 * n + 1; steps++; } return steps; }
let best = 0; for (let i = 1; i < 2000; i++) { const c = collatz(i); if (c > best) best = c; }
console.log(best);
function poly(x) { return 3 * x * x - 2 * x + 1; }
let acc = 0; for (let i = 0; i < 1000; i++) acc = acc + poly(i * 0.5); console.log(acc);
function gcd(a, b) { while (b !== 0) { const t = b; b = a % b; a = t; } return a; }
console.log(gcd(1071, 462), gcd(17, 5), gcd(0, 9));
