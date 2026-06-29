function f47(a, b) { let v0 = 4; let v1 = 5; let v2 = 2; for (let i = 0; i !== a; i = i + 1) { v2 = 3 - a; v2 = v0 + v0; v1 = v2 - v2; if (v1 > 34807) v1 = v1 % 195; v0 = 3 * a; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f47(k % 7, k);
console.log(acc);
console.log(f47(5, 3));
console.log(f47(20, 2147483));
console.log(f47(3, 0.5));
console.log(f47(4, -7.25));
console.log(f47(12, 1000000.0));
console.log(f47(0, 9));
console.log(f47(30, 65536));
