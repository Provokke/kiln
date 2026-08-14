function r1(n) { if (n < 4) return n * 1 + 1; return (r1(n - 1) + r1(n - 2) + r1(n - 3) + r1(n - 4)) % 34454; }
console.log(r1(0));
console.log(r1(3));
console.log(r1(8));
console.log(r1(18));
