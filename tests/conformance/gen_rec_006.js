function r6(n) { if (n < 3) return n * 3 + 0; return (r6(n - 1) + r6(n - 2)) % 92740; }
console.log(r6(0));
console.log(r6(3));
console.log(r6(8));
console.log(r6(13));
