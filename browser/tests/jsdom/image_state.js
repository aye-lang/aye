const PNG = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAIAAAAECAYAAACk7+45AAAAEUlEQVR4nGP4z8DwH4QZcDMAHxMP8b9IbMkAAAAASUVORK5CYII=';
const BAD = 'data:image/png;base64,bm90IGEgcG5n';
const output = document.getElementById('results');
function check(name, ok) { output.append(name + ': ' + (ok ? 'PASS' : 'FAIL') + '\n'); }
function task() { return new Promise(resolve => setTimeout(resolve, 0)); }
async function fails(promise) {
  try { await promise; return false; } catch (e) { return e instanceof DOMException && e.name === 'EncodingError'; }
}
async function run() {
  const parsed = document.getElementById('parsed');
  let parsedLoads = 0;
  parsed.onload = () => { parsedLoads++; check('parsed load state', parsed.complete && parsed.naturalWidth === 2); };
  const img = new Image(10, 20);
  check('Image constructor', img instanceof Image && img instanceof HTMLImageElement && img.width === 10 && img.height === 20);
  check('initial state', img.complete && img.naturalWidth === 0 && img.naturalHeight === 0 && img.currentSrc === '');
  const created = document.createElement('img');
  check('createElement initial', created.complete && created.width === 0 && created.height === 0);
  check('decode without source', await fails(created.decode()));
  await parsed.decode();
  await task();
  check('parsed loads once', parsedLoads === 1);
  let loads = 0, errors = 0, eventStates = true;
  img.onload = () => { loads++; eventStates = eventStates && img.complete && img.naturalWidth > 0; };
  img.onerror = () => { errors++; eventStates = eventStates && img.complete && img.naturalWidth === 0 && img.naturalHeight === 0; };
  img.src = PNG;
  check('source starts pending', !img.complete && img.naturalWidth === 0 && loads === 0);
  const promise = img.decode();
  check('decode is Promise', promise instanceof Promise);
  await promise;
  check('decoded before load task', img.complete && img.naturalWidth === 2 && img.naturalHeight === 4 && img.currentSrc === PNG && loads === 0);
  check('attribute dimensions', img.width === 10 && img.height === 20);
  img.removeAttribute('width'); img.setAttribute('height', 'bogus');
  check('natural dimension fallback', img.width === 2 && img.height === 4);
  img.width = 7; img.height = 8;
  check('dimension setters', img.getAttribute('width') === '7' && img.getAttribute('height') === '8');
  await task();
  check('load once', loads === 1 && errors === 0);
  img.src = BAD;
  check('replacement pending', !img.complete && img.naturalWidth === 0 && img.currentSrc === '');
  check('broken decode', await fails(img.decode()));
  check('broken state', img.complete && img.naturalWidth === 0 && img.naturalHeight === 0 && img.currentSrc === BAD);
  await task();
  check('error once', loads === 1 && errors === 1);
  img.src = '';
  check('empty source complete', img.complete && img.currentSrc === '' && img.naturalWidth === 0);
  check('empty decode', await fails(img.decode()));
  await task();
  check('empty error once', errors === 2);
  img.removeAttribute('src');
  check('removed source complete', img.complete);
  check('removed decode', await fails(img.decode()));
  await task();
  check('removed source no event', loads === 1 && errors === 2);
  img.src = PNG;
  const stale = img.decode();
  img.src = BAD;
  img.src = PNG;
  check('superseded decode', await fails(stale));
  await Promise.all([img.decode(), img.decode()]);
  await task();
  check('rapid updates coalesce', loads === 2 && errors === 2 && img.currentSrc === PNG);
  img.src = PNG;
  await img.decode();
  // Change again after decoding but before the previous event task.
  img.src = BAD;
  check('changed after decode', await fails(img.decode()));
  await task();
  check('stale event canceled', loads === 2 && errors === 3);
  created.srcset = PNG + ' 2x';
  check('srcset pending', !created.complete);
  await created.decode();
  check('srcset natural density', created.currentSrc === PNG && created.naturalWidth === 1 && created.naturalHeight === 2 && created.width === 1 && created.height === 2);
  const host = document.createElement('div');
  host.innerHTML = '<img src="' + PNG + '">';
  const markup = host.firstChild;
  let markupLoads = 0;
  markup.addEventListener('load', () => markupLoads++);
  await markup.decode();
  await task();
  check('detached markup request', markupLoads === 1 && markup.complete && markup.naturalWidth === 2);
  markup.attributes.getNamedItem('src').value = BAD;
  check('Attr restarts request', !markup.complete && await fails(markup.decode()));
  await task();
  check('event state ordering', eventStates);
  check('final event counts', loads === 2 && errors === 3);
}
run().catch(e => output.append('UNEXPECTED: ' + e.name + ' ' + e.message + '\n'));
