const obj = {
  val: 42,
  regular() { return this.val; },
  arrow() { const f = () => this.val; return f(); },
  nested() { return [1, 2].map(x => x * this.val); },
};
console.log(obj.regular(), obj.arrow(), obj.nested());
class T { constructor() { this.v = 1; this.get = () => this.v; } }
const t = new T(); const g = t.get; console.log(g());
function Old(x) { this.x = x; }
console.log(new Old(5).x, new Old(5));
