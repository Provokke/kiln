function f17(a, b) { let v0 = 0; let v1 = 0; for (let i = 0; i < a; i = i + 1) { v1 = 4 + v1; v0 = b + v1; v1 = a + v0; if (v1 > 15581) v1 = v1 % 463; v0 = 2 - v0; if (v0 > 78034) v0 = v0 % 650; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f17(k % 7, k);
console.log(acc);
console.log(f17(5, 3));
console.log(f17(20, 2147483));
console.log(f17(3, 0.5));
console.log(f17(4, -7.25));
console.log(f17(12, 1000000.0));
console.log(f17(0, 9));
console.log(f17(30, 65536));
