import assert from 'node:assert/strict';
import { test } from 'node:test';
import { renderToStaticMarkup } from 'react-dom/server';
import { App } from '../App';
import { ArmyEditor } from '../components/ArmyEditor';
import { PlayersPanel } from '../components/PlayersPanel';
import { TownBuildings } from '../components/TownBuildings';
import { Feedback } from './Feedback';
import { ReasonText } from './reasons';
import { i18n, t } from './index';
import { languages } from './locale';
import type { ArmySlotDto, PlayerDto } from '../types/api';

test('four-language shell, editor and error rendering keeps identities, classes and dirty markers', async () => {
  const stack: ArmySlotDto = { creatureId: 0, count: 1234, name: '<Custom & Иван>', empty: false, editable: true };
  const player = {
    id: 0, color: 0, name: 'Red', status: 'active', statusReason: 'technical detail',
    heroCount: 1, townCount: 1, resources: [1, 2, 3, 4, 5, 6, 7], original: [1, 2, 3, 4, 5, 6, 7],
    dirtyResources: [], readOnlyReason: null, details: { startingHero: null, startingTown: null, team: null, eligibility: null,
      allies: null, enemies: null, mapSize: null, obelisks: null },
  } as unknown as PlayerDto;
  for (const language of languages) {
    await i18n.changeLanguage(language);
    const shell = renderToStaticMarkup(<App />);
    assert.ok(shell.includes(t('common.settings')));
    assert.ok(shell.includes(t('common.theme')));
    assert.ok(shell.includes(t('common.themeLight')));
    assert.ok(shell.includes(t('common.themeDark')));
    assert.ok(shell.includes(t('common.themeSystem')));
    assert.ok(shell.includes(t('ui.openSave')));
    const army = renderToStaticMarkup(<ArmyEditor slots={[stack]} choices={[]} readOnlyReason={null}
      dirtyFields={new Set(['Army slot 1'])} onApplyEdit={() => { throw new Error('render must not edit'); }} />);
    assert.ok(army.includes('army-stack-dirty'));
    assert.ok(army.includes('value="1234"'));
    const players = renderToStaticMarkup(<PlayersPanel players={[player]} selectedId={0} onSelect={() => {}}
      onEdit={() => { throw new Error('render must not edit'); }} onRevert={() => {}}
      epoch={0} artworkVersion="" busy={false} encoding="auto" mapLinkEpoch={0} />);
    assert.ok(players.includes('id="tab-Resources"'));
    assert.ok(players.includes('id="player-Resources"'));
    assert.ok(players.includes(t('common.resources')));
    const town = renderToStaticMarkup(<TownBuildings buildings={null} onApplyEdit={() => {}} />);
    assert.ok(town.includes(t('ui.buildingStateIsDecodedOnlyForHota4411')));
    const feedback = renderToStaticMarkup(<Feedback message={{ savedFile: '<Custom & Иван>' }} />);
    assert.ok(feedback.includes('&lt;Custom &amp; Иван&gt;'));
    const reason = renderToStaticMarkup(<ReasonText reason={{ code: 'read-only', details: '<unknown bytes>' }} />);
    assert.ok(reason.includes(t('ui.readOnly')));
    assert.ok(reason.includes('&lt;unknown bytes&gt;'));
    for (const html of [shell, army, players, town, feedback, reason]) {
      assert.ok(!html.includes('[object Object]'));
      assert.ok(!html.includes('{{'));
      assert.ok(!html.includes('ui.'));
    }
  }
  await i18n.changeLanguage('en');
});

// This is a compile-time check only; an unknown key must not become valid.
test('compact garrison keeps seven accessible editable slots, values and dirty markers', () => {
  const slots: ArmySlotDto[] = Array.from({ length: 7 }, (_, i) => ({ creatureId: i, count: 100 + i,
    name: `Creature ${i}`, empty: false, editable: true }));
  const html = renderToStaticMarkup(<ArmyEditor compact slots={slots}
    choices={slots.map(s => ({ id: s.creatureId, name: s.name! }))} readOnlyReason={null}
    dirtyPrefix="Garrison slot" dirtyFields={new Set(['Garrison slot 7'])}
    onApplyEdit={() => { throw new Error('render must not edit'); }} />);
  assert.ok(html.includes('army-grid-compact'));
  assert.equal((html.match(/role="group"/g) ?? []).length, 7);
  assert.equal((html.match(/type="number"/g) ?? []).length, 7);
  assert.equal((html.match(/aria-haspopup="listbox"/g) ?? []).length, 7);
  assert.equal((html.match(/army-stack-dirty/g) ?? []).length, 1);
  assert.ok(!html.includes('army-field-caption'));
  assert.ok(!html.includes(t('ui.armyChangesBecomePendingEditsImmediatelyNothingIsWritten')));
  for (let i = 0; i < 7; i++) assert.ok(html.includes(`value="${100 + i}"`));
});

if (false) {
  // @ts-expect-error nonexistent keys must be rejected
  t('missing.key');
}
