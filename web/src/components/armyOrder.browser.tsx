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
    for (const width of [280, 360, 560, 620, 1200]) {
      flushSync(() => root.render(<div style={{ width }}>
        <ArmyEditor key={`${compact}-${width}`} compact={compact} slots={slots} choices={choices}
          readOnlyReason={null} dirtyFields={new Set()} onApplyEdit={edit => edits.push(edit)} />
      </div>));
      window.scrollTo(0, 0);
      await settle();
      const grid = document.querySelector<HTMLElement>('.army-grid')!;
      const cards = [...grid.querySelectorAll<HTMLElement>('.army-stack')];
      check(cards.length === 7, 'Seven army slots');
      const columns = width < 560 ? 1 : 2;
      const first = cards[0]!.getBoundingClientRect();
      const trigger = cards[0]!.querySelector<HTMLButtonElement>('.creature-combobox-trigger')!;
      trigger.click();
      await settle();
      const search = document.querySelector<HTMLInputElement>('.creature-combobox-search')!;
      check(search !== null, 'Searchable selector opens');
      const popup = document.querySelector<HTMLElement>('.creature-combobox-popup')!.getBoundingClientRect();
      const anchor = trigger.getBoundingClientRect();
      check(Math.min(Math.abs(popup.top - anchor.bottom), Math.abs(popup.bottom - anchor.top)) <= 8, 'Popup stays anchored to its selector');
      trigger.click();
      await settle();
      for (const [index, card] of cards.entries()) {
        const rect = card.getBoundingClientRect();
        if (index % columns === 0) {
          check(Math.abs(rect.left - first.left) < 1, 'Each row starts in the first column');
          if (index) check(rect.top > cards[index - columns]!.getBoundingClientRect().bottom, 'Rows advance in binary slot order');
        } else {
          const previous = cards[index - 1]!.getBoundingClientRect();
          check(Math.abs(rect.top - previous.top) < 1 && rect.left > previous.right, 'Second slot follows on the same row');
        }
        check(card.getAttribute('aria-label') === `Army slot ${index + 1}`, 'Logical slot label');
        check(card.textContent?.includes(army[index]![1]), 'Creature identity stays with slot');
        const input = card.querySelector<HTMLInputElement>('input')!;
        const selector = card.querySelector<HTMLElement>('.creature-combobox-trigger')!;
        check(selector.getBoundingClientRect().right <= input.getBoundingClientRect().left, 'Selector and quantity fit side by side');
        check(card.scrollWidth <= card.clientWidth, 'Slot has no horizontal overflow');
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
      check(grid.scrollWidth <= grid.clientWidth, 'No horizontal grid scrolling');
      check(Math.abs(cards[6]!.getBoundingClientRect().left - first.left) < 1, 'Slot 7 occupies the first column of the last row');
    }
  }
  return { ok: true, assertions, edits: edits.length };
}
run().then(result => { document.getElementById('result')!.textContent = JSON.stringify(result); })
  .catch(error => { document.getElementById('result')!.textContent = JSON.stringify({ ok: false, error: String(error), stack: error.stack }); });
