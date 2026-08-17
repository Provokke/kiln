function r5(n) { if (n < 2) return n * 1 + 0; return (r5(n - 1) + r5(n - 2) + r5(n - 3) + r5(n - 4)) % 36079; }
console.log(r5(0));
console.log(r5(3));
console.log(r5(8));
console.log(r5(16));
