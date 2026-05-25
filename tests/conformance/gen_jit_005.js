function f5(a, b) { let v0 = 4; let v1 = 1; for (let i = 0; i < a; i = i + 1) { v1 = v0 * 5; if (v1 > 95567) v1 = v1 % 500; v1 = 3 + a; v1 = v0 * a; v1 = v0 + 6; v1 = v1 - v0; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f5(k % 7, k);
console.log(acc);
console.log(f5(5, 3));
console.log(f5(20, 2147483));
console.log(f5(3, 0.5));
console.log(f5(4, -7.25));
console.log(f5(12, 1000000.0));
console.log(f5(0, 9));
console.log(f5(30, 65536));
