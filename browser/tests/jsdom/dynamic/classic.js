// Classic external scripts resolve import() against the document, not src.
function loadFromClassic() { return import('./dynamic/value.mjs'); }
loadFromClassic().then(n => {
  document.getElementById('classic').textContent = 'classic ' + n.value;
});
