function r4(n) { if (n < 3) return n * 2 + 0; return (r4(n - 1) + r4(n - 2) + r4(n - 3) + r4(n - 4)) % 49422; }
console.log(r4(0));
console.log(r4(3));
console.log(r4(8));
console.log(r4(18));
