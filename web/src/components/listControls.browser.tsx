// Browser acceptance fixture: synthetic DTOs only; no save files or native writes.
import { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { HeroList } from './HeroList';
import { TownList } from './TownList';
import { PlayersPanel } from './PlayersPanel';
import { CreaturesPanel } from './CreaturesPanel';
import { i18n, t } from '../i18n';
import type { HeroListItemDto, TownListItemDto, PlayerDto, NeutralArmyDto } from '../types/api';
import '../styles/tokens.css';
import '../styles/globals.css';

const raw = (name: string) => ({ raw: [...new TextEncoder().encode(name)], lossy: name });
const heroes: HeroListItemDto[] = Array.from({ length: 40 }, (_, id) => ({ id,
  name: raw(['Яна', 'Adam', 'Борис'][id % 3]!), owner: { kind: id === 2 ? 'neutral' : 'player', player: id === 2 ? null : id % 2, label: '' },
  level: id + 1, armyCreatureIds: [], confidence: 'high', hasSpellbook: false, hasNotes: false, duplicateName: false,
}));
const towns: TownListItemDto[] = heroes.map(hero => ({ ...hero, townType: hero.id % 2, townTypeName: hero.id % 2 ? 'Rampart' : 'Castle',
  garrisonCreatureIds: [], hasGarrisonHero: hero.id === 0, hasVisitingHero: false, hasFort: false,
}));
const players = Array.from({ length: 8 }, (_, id) => ({ id, color: id, name: 'Unverified personal name', status: id === 0 ? 'active' : 'unknown',
  active: id === 0, statusReason: '', heroCount: id === 0 ? 1 : 0, townCount: 0,
  resources: [10, 2, 3, 4, 5, 6, 7], original: [1, 2, 3, 4, 5, 6, 7], dirtyResources: [0],
  tentVisits: Array(8).fill(false), originalTentVisits: Array(8).fill(false), dirtyTents: [], readOnlyReason: null,
  team: null, originalTeam: null, teamCount: 0, teamOptions: [], teamState: 'unsupported', teamDirty: false,
  details: { mapLinked: id < 4, team: id < 2 ? 1 : null, startingHero: null, startingTown: null, eligibility: null,
    allies: null, enemies: null, mapSize: null, obelisks: null },
})) as unknown as PlayerDto[];
const groups: NeutralArmyDto[] = [0, 1, 2].map(id => ({ id, kind: id === 2 ? 'garrison' : 'wandering',
  creatureId: id, name: ['Dragon', 'Архангел', 'Pikeman'][id]!, position: [id, 2, id % 2], owner: id === 2 ? 255 : null,
  army: [{ creatureId: id, name: ['Dragon', 'Архангел', 'Pikeman'][id]!, count: id + 10, empty: false, editable: true }],
  dirtySlots: [0], allowedCreatures: [id], initialArmy: null, readOnlyReason: null,
}));
let writes = 0;
Object.assign(window, { isTauri: true, __TAURI_INTERNALS__: { invoke: async (command: string) => {
  if (/apply|discard|save/.test(command)) { writes++; throw new Error('Unexpected native write'); }
  if (command === 'list_neutral_armies') return { groups, choices: [], unavailableReason: null, initialGroups: null };
  throw new Error('No artwork in synthetic fixture');
} } });

function Fixture({ section }: { section: string }) {
  const [selected, setSelected] = useState<number | null>(0);
  return <div className="overview-columns" style={{ height: 600, margin: 12 }}>
    {section === 'heroes' ? <HeroList heroes={heroes} selectedId={selected} onSelect={setSelected} encoding="auto" portraits={new Map()} />
      : section === 'towns' ? <TownList towns={towns} selectedId={selected} onSelect={setSelected} encoding="auto" />
      : section === 'players' ? <PlayersPanel players={players} selectedId={selected ?? 0} onSelect={setSelected} onEdit={() => { writes++; }} onTentEdit={() => { writes++; }} onTeamEdit={() => { writes++; }} onRevert={() => { writes++; }} epoch={0} artworkVersion="" busy={false} encoding="auto" mapLinkEpoch={0} />
      : <CreaturesPanel epoch="fixture" onChanged={async () => {}} onBusy={() => {}} />}
    {(section === 'heroes' || section === 'towns') && <div data-testid="detail"><span>{selected}</span><input aria-label="Draft" defaultValue="pending draft" /></div>}
  </div>;
}

const root = createRoot(document.getElementById('root')!);
const settle = () => new Promise(resolve => setTimeout(resolve, 35));
let assertions = 0;
function check(value: unknown, message: string) { assertions++; if (!value) throw new Error(message); }
function find<T extends Element>(selector: string): T { const node = document.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; }
function clickText(text: string) { const node = [...document.querySelectorAll<HTMLButtonElement>('button')].find(button => button.textContent?.trim() === text); if (!node) throw new Error(`Missing button ${text}`); node.click(); }
function input(selector: string, value: string) {
  const node = find<HTMLInputElement>(selector);
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!.call(node, value);
  node.dispatchEvent(new Event('input', { bubbles: true }));
}
function select(value: string) { const node = find<HTMLSelectElement>('.list-controls select'); node.value = value; node.dispatchEvent(new Event('change', { bubbles: true })); }
async function mount(section: string) { flushSync(() => root.render(<Fixture key={section} section={section} />)); await settle(); }
async function openFilters() { find<HTMLButtonElement>('.list-controls button[aria-controls]').click(); await settle(); check(find('.list-filter-popover').matches(':popover-open'), 'Filter opens'); }

async function run() {
  localStorage.clear();
  await mount('heroes');
  const scroll = find<HTMLUListElement>('.hero-rows'); scroll.scrollTop = 100;
  select('name'); await settle();
  check(scroll === find('.hero-rows') && scroll.scrollTop === 100, 'Sorting preserves list node and scroll');
  check(find('[data-testid="detail"] span').textContent === '0', 'Sorting preserves selection');
  await openFilters();
  const checks = document.querySelectorAll<HTMLInputElement>('.list-facet input');
  checks[0]!.click(); checks[1]!.click(); await settle();
  check(checks[0]!.checked && checks[1]!.checked, 'Two owner colors selected');
  check(document.querySelectorAll('.hero-row').length === 39, 'Owner OR excludes only neutral');
  find<HTMLElement>('.list-filter-popover').hidePopover();
  input('input[type="search"]', 'Adam'); await settle();
  check(document.querySelectorAll('.hero-row').length === 13, 'Search combines with owner filters');
  check(find('[data-testid="detail"] input').getAttribute('value') === 'pending draft', 'Hidden selection retains draft');
  check(find('.list-filter-summary').textContent?.includes(t('lists.hidden')), 'Hidden-selection notice');
  await openFilters();
  check(checks[0]!.checked && checks[1]!.checked, 'Closing preserves active filters');
  check(find('.list-controls button[aria-controls]').textContent === `${t('lists.filter')} (2)`, 'Badge excludes independent search');
  clickText(t('lists.clearAll')); await settle();
  check(document.querySelectorAll('.hero-row').length === 13, 'Clear preserves search results');
  check(find<HTMLInputElement>('input[type="search"]').value === 'Adam', 'Clear retains search');
  check(find<HTMLSelectElement>('.list-controls select').value === 'name', 'Clear retains sort');
  check(!checks[0]!.checked && !checks[1]!.checked, 'Clear unchecks colors immediately');
  check(find('.list-controls button[aria-controls]').textContent === t('lists.filter'), 'Clear removes filter badge');
  check(find('.list-filter-popover').matches(':popover-open'), 'Clear keeps popup open for refinement');
  find<HTMLElement>('.list-filter-popover').hidePopover();
  input('input[type="search"]', ''); await settle();
  check(document.querySelectorAll('.hero-row').length === 40, 'Independent search clearing restores all results');

  await mount('towns');
  find<HTMLInputElement>('.owned-filter input').click(); await settle();
  check(document.querySelectorAll('.hero-row').length === 1, 'Hero inside preserved');
  await openFilters();
  document.querySelectorAll<HTMLInputElement>('.list-facet')[1]!.querySelectorAll<HTMLInputElement>('input')[1]!.click(); await settle();
  check(document.querySelectorAll('.hero-row').length === 0, 'Faction AND Hero inside');
  check(find('[data-testid="detail"] span').textContent === '0', 'Town hidden selection retained');
  clickText(t('lists.clearAll')); await settle();
  check(document.querySelectorAll('.hero-row').length === 40, 'Town clear removes faction and quick filters');
  check(!find<HTMLInputElement>('.owned-filter input').checked, 'Town quick filter unchecked');
  find<HTMLElement>('.list-filter-popover').hidePopover();

  await mount('players');
  const draft = find<HTMLInputElement>('.player-resource-input input'); draft.focus();
  input('.player-resource-input input', 'invalid draft'); await settle();
  await openFilters();
  document.querySelectorAll<HTMLInputElement>('.list-facet input')[1]!.click(); await settle();
  check(document.querySelectorAll('.hero-row').length === 7, 'Unknown player status filter');
  check(find<HTMLInputElement>('.player-resource-input input') === draft && draft.value === 'invalid draft', 'Hidden player keeps editor input');
  check(document.querySelector('.resource-dirty'), 'Pending player marker preserved');
  find<HTMLElement>('.list-filter-popover').hidePopover(); await settle(); await openFilters();
  check(document.querySelectorAll<HTMLInputElement>('.list-facet input')[1]!.checked, 'Player filter survives close');
  clickText(t('lists.clearAll')); await settle();
  check(document.querySelectorAll('.hero-row').length === 8, 'Player clear restores all statuses');
  check(draft.value === 'invalid draft', 'Player clear preserves pending editor input');
  find<HTMLElement>('.list-filter-popover').hidePopover();

  await mount('creatures');
  check(document.querySelectorAll('.hero-row').length === 2, 'Default wandering category');
  const army = find<HTMLInputElement>('.army-quantity-field input');
  input('.army-quantity-field input', ''); await settle();
  input('.hero-list input[type="search"]', 'Архангел'); await settle();
  check(document.querySelectorAll('.hero-row').length === 1, 'Creature search');
  check(find('.neutral-detail h2').textContent === 'Dragon', 'Creature selection independent of results');
  check(find<HTMLInputElement>('.army-quantity-field input') === army && army.value === '', 'Creature draft survives hidden selection');
  await openFilters(); clickText(t('lists.clearAll')); await settle();
  check(document.querySelectorAll('.hero-row').length === 1, 'Creature clear keeps independent search');
  check(find<HTMLInputElement>('.hero-list input[type="search"]').value === 'Архангел', 'Creature search retained');
  input('.hero-list input[type="search"]', ''); await settle();
  check(document.querySelectorAll('.hero-row').length === 3, 'Clear includes all categories');
  document.querySelectorAll<HTMLInputElement>('.list-facet input[type="checkbox"]')[8]!.click(); await settle();
  check(document.querySelectorAll('.hero-row').length === 1, 'Known neutral separated from null owner');
  check(find('.neutral-detail h2').textContent === 'Dragon', 'Neutral filter retains selected creature');
  find<HTMLElement>('.list-filter-popover').hidePopover();

  // Real browser layout in every catalog and theme, at the supplied 1280x720 viewport.
  for (const language of ['en', 'ru', 'de', 'es']) {
    await i18n.changeLanguage(language);
    for (const theme of ['light', 'dark']) {
      document.documentElement.dataset.theme = theme;
      for (const section of ['heroes', 'towns', 'players', 'creatures']) {
        await mount(section); await openFilters();
        const panel = find('.list-filter-popover').getBoundingClientRect();
        check(panel.left >= 0 && panel.right <= innerWidth && panel.bottom <= innerHeight, `${section}/${language}/${theme} popover fits`);
        check(find('.hero-rows').getBoundingClientRect().height > 200, `${section} retains list space`);
        check(panel.width <= 256 && panel.height <= 360, 'Popup remains compact');
        check(panel.left >= find('.hero-list').getBoundingClientRect().right + 8, 'Popup opens to the right without covering list rows');
        check(find('.list-filter-popover').querySelectorAll('button').length === 1, 'Only Clear all action in popup');
        check(!document.querySelector('.list-filter-summary button'), 'No duplicate external clear action');
        check(document.documentElement.scrollWidth <= innerWidth, 'No horizontal viewport overflow');
        check(find('.list-filter-popover').contains(document.activeElement), 'Keyboard focus enters filters');
        find<HTMLElement>('.list-filter-popover').hidePopover(); await settle();
        check(document.activeElement === find('.list-controls button[aria-controls]'), 'Focus returns on dismissal');
      }
    }
  }
  check(writes === 0, 'Presentation interactions issue no edit commands');
  await mount('heroes');
  select('name'); await settle();
  find<HTMLButtonElement>('.list-direction').click(); await settle();
  await mount('towns'); await mount('heroes');
  check(find<HTMLSelectElement>('.list-controls select').value === 'name' && find('.list-direction').textContent === '↓', 'Sort survives remount');
  check(document.querySelectorAll('.hero-row').length === 40, 'Filters do not persist to remount');
  await openFilters();
  document.getElementById('result')!.textContent = JSON.stringify({ ok: true, assertions, writes });
}
void run().catch(error => { document.getElementById('result')!.textContent = JSON.stringify({ ok: false, assertions, error: String(error), stack: error.stack }); });
