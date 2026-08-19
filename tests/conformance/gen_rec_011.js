function r11(n) { if (n < 3) return n * 2 + 2; return (r11(n - 1) + r11(n - 2) + r11(n - 3)) % 25891; }
console.log(r11(0));
console.log(r11(3));
console.log(r11(8));
console.log(r11(14));
