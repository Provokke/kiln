function f53(a, b) { let v0 = 4; let v1 = 4; for (let i = 0; i <= a; i = i + 1) { v1 = v1 + a; v1 = b + b; v1 = v0 - a; if (v1 > 20560) v1 = v1 % 599; v0 = 1 + b; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f53(k % 7, k);
console.log(acc);
console.log(f53(5, 3));
console.log(f53(20, 2147483));
console.log(f53(3, 0.5));
console.log(f53(4, -7.25));
console.log(f53(12, 1000000.0));
console.log(f53(0, 9));
console.log(f53(30, 65536));
