function f44(a, b) { let v0 = 0; let v1 = 1; let v2 = 1; for (let i = 0; i < a; i = i + 1) { v1 = v1 - a; v0 = v1 - 5; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f44(k % 7, k);
console.log(acc);
console.log(f44(5, 3));
console.log(f44(20, 2147483));
console.log(f44(3, 0.5));
console.log(f44(4, -7.25));
console.log(f44(12, 1000000.0));
console.log(f44(0, 9));
console.log(f44(30, 65536));
