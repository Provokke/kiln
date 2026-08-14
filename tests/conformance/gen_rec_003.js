function r3(n) { if (n < 4) return n * 2 + 0; return (r3(n - 1) + r3(n - 2) + r3(n - 3)) % 22837; }
console.log(r3(0));
console.log(r3(3));
console.log(r3(8));
console.log(r3(17));
