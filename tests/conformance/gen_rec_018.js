function r18(n) { if (n < 3) return n * 3 + 1; return (r18(n - 1) + r18(n - 2) + r18(n - 3) + r18(n - 4)) % 13698; }
console.log(r18(0));
console.log(r18(3));
console.log(r18(8));
console.log(r18(16));
