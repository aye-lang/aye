const inputs = ['{bad', '', '[1,]', "{'a':1}", '{"a":1}', 'undefined', 'NaN', '01', '"unterminated', '[1, 2]', 'true', ' null '];
for (const s of inputs) {
  try {
    const v = JSON.parse(s);
    console.log('ok', JSON.stringify(v));
  } catch (e) {
    console.log(e.name, e instanceof SyntaxError);
  }
}
