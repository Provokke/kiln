function r16(n) { if (n < 4) return n * 3 + 0; return (r16(n - 1) + r16(n - 2) + r16(n - 3)) % 91913; }
console.log(r16(0));
console.log(r16(3));
console.log(r16(8));
console.log(r16(13));
