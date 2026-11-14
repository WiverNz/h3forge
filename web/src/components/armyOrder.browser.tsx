// Xyron's actual SaAndVla army; no private save bytes or artwork are bundled.
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { ArmyEditor, type ArmyEditRequest } from './ArmyEditor';
import { i18n } from '../i18n';
import '../styles/tokens.css';
import '../styles/globals.css';

const army = [
  [43, 'Familiar', 1418], [45, 'Magog', 5080], [47, 'Cerberus', 6880],
  [49, 'Horned Demon', 230], [51, 'Pit Lord', 116],
  [53, 'Efreet Sultan', 5008], [55, 'Arch Devil', 2251],
] as const;
const slots = army.map(([creatureId, name, count]) => ({ creatureId, name, count, empty: false, editable: true }));
const choices = slots.map(({ creatureId, name }) => ({ id: creatureId, name }));
const root = createRoot(document.getElementById('root')!);
const edits: ArmyEditRequest[] = [];
const settle = () => new Promise(resolve => setTimeout(resolve, 20));
let assertions = 0;
function check(value: unknown, message: string) { assertions++; if (!value) throw new Error(message); }

async function run() {
  await i18n.changeLanguage('en');
  for (const compact of [false, true]) {
    for (const width of [360, 620, 1200]) {
      flushSync(() => root.render(<div style={{ width }}>
        <ArmyEditor key={`${compact}-${width}`} compact={compact} slots={slots} choices={choices}
          readOnlyReason={null} dirtyFields={new Set()} onApplyEdit={edit => edits.push(edit)} />
      </div>));
      await settle();
      const grid = document.querySelector<HTMLElement>('.army-grid')!;
      const cards = [...grid.querySelectorAll<HTMLElement>('.army-stack')];
      check(cards.length === 7, 'Seven army slots');
      const first = cards[0]!.getBoundingClientRect();
      for (const [index, card] of cards.entries()) {
        const rect = card.getBoundingClientRect();
        check(Math.abs(rect.top - first.top) < 1, `Slot ${index + 1} stays on the same row`);
        if (index) check(rect.left > cards[index - 1]!.getBoundingClientRect().right, 'Strict left-to-right order');
        check(card.getAttribute('aria-label') === `Army slot ${index + 1}`, 'Logical slot label');
        check(card.textContent?.includes(army[index]![1]), 'Creature identity stays with slot');
        const input = card.querySelector<HTMLInputElement>('input')!;
        check(input.value === String(army[index]![2]), 'Actual Xyron quantity');
        // Exercise the real React input/blur path, including the zero-based request.
        input.focus();
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!.call(input, String(army[index]![2] + 1));
        input.dispatchEvent(new Event('input', { bubbles: true }));
        await settle();
        const before = edits.length;
        input.blur();
        await settle();
        check(edits.length === before + 1, 'One immediate blur edit');
        check(JSON.stringify(edits.at(-1)?.army) === JSON.stringify({ slot: index, creatureId: army[index]![0], count: army[index]![2] + 1 }), 'Edit targets the displayed slot');
      }
      grid.scrollLeft = grid.scrollWidth;
      check(cards[6]!.getBoundingClientRect().right <= grid.getBoundingClientRect().right + 1, 'Seventh slot is reachable');
    }
  }
  return { ok: true, assertions, edits: edits.length };
}
run().then(result => { document.getElementById('result')!.textContent = JSON.stringify(result); })
  .catch(error => { document.getElementById('result')!.textContent = JSON.stringify({ ok: false, error: String(error), stack: error.stack }); });
