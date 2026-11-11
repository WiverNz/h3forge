import { ReasonText, type Reason } from '../i18n/reasons';
import { t } from '../i18n/index';
import { replaceSkill } from '../lib/skillReplacement';
import { SkillIcon } from './SkillIcon';
import { SkillCombobox } from './SkillCombobox';
import { useState } from 'react';
import type { HeroEditRequest, HeroSkillDto } from '../types/api';

const MASTERIES = [
  { value: 1, get label() { return t('ui.basic'); } },
  { value: 2, get label() { return t('ui.advanced'); } },
  { value: 3, get label() { return t('ui.expert'); } },
];

/**
 * Editable secondary-skill list, immediate-edit model like attributes and
 * armies: every control dispatches the absolute ordered skill list as a
 * pending edit; nothing is written to disk until Save or Save As. Choices are
 * displayed alphabetically but carry their binary skill IDs.
 */
export function SkillsEditor({ skills, choices, readOnlyReason, dirty, onApplyEdit }: {
  skills: HeroSkillDto[];
  choices: { id: number; name: string | null }[];
  readOnlyReason: Reason | null;
  dirty: boolean;
  onApplyEdit: (request: HeroEditRequest) => void;
}) {
  const [pendingMastery, setPendingMastery] = useState(1);

  if (readOnlyReason) {
    return <>
      <p className="fineprint">{<ReasonText reason={readOnlyReason} />}</p>
      {skills.length > 0 ? (
        <ul className="chip-list">
          {skills.map((skill) => (
            <li key={skill.skillId} className="chip">
              <SkillIcon id={skill.skillId} mastery={skill.mastery} />{skill.name ?? t('ui.skillValue', { value1: skill.skillId })}
              <span className="chip-sub">{skill.masteryName ?? t('ui.masteryValue', { value1: skill.mastery })}</span>
            </li>
          ))}
        </ul>
      ) : (
        <p>{t('ui.noneRecorded')}</p>
      )}
    </>;
  }

  const owned = new Set(skills.map((skill) => skill.skillId));
  const dispatch = (next: { id: number; mastery: number }[]) => onApplyEdit({ skills: next });
  const asPairs = () => skills.map((skill) => ({ id: skill.skillId, mastery: skill.mastery }));
  const available = choices
    .filter((choice) => !owned.has(choice.id))
    .slice()
    .sort((a, b) => (a.name ?? '').localeCompare(b.name ?? ''));

  return <div className="skills-editor">
    <div className="skills-head">
      <h4 className="skills-title">{t('ui.secondarySkills')}{dirty && <span className="dirty-dot" title={t('ui.unsavedChange')} />}</h4>
      <span className="fineprint">{skills.length}/28</span>
    </div>
    {skills.length === 0 && <p className="fineprint">{t('ui.noSkillsAddOneBelow')}</p>}
    <ul className="skill-rows">
      {skills.map((skill, index) => <li key={skill.skillId} className="skill-row">
        <SkillCombobox value={skill.skillId} mastery={skill.mastery}
          choices={choices.filter((choice) => choice.id === skill.skillId || !owned.has(choice.id))
            .slice().sort((a, b) => (a.name ?? '').localeCompare(b.name ?? ''))}
          onChange={(id) => dispatch(replaceSkill(skills, index, id, choices.map((choice) => choice.id)))} />
        <label className="skill-mastery-label">{t('ui.mastery')}<select className="skill-mastery" aria-label={t('ui.masteryForValue', { value1: skill.name ?? skill.skillId })}
            value={skill.mastery}
            onChange={(event) => dispatch(asPairs().map((pair) =>
              pair.id === skill.skillId ? { id: pair.id, mastery: Number(event.target.value) } : pair))}>
            {MASTERIES.map((mastery) => <option key={mastery.value} value={mastery.value}>
              {mastery.label}</option>)}
          </select>
        </label>
        <div className="skill-row-actions">
          <button type="button" className="skill-move" aria-label={t('ui.moveValueUp', { value1: skill.name ?? skill.skillId })}
            title={t('ui.moveUp')} disabled={index === 0}
            onClick={() => {
              const pairs = asPairs();
              [pairs[index - 1], pairs[index]] = [pairs[index]!, pairs[index - 1]!];
              dispatch(pairs);
            }}>↑</button>
          <button type="button" className="skill-move" aria-label={t('ui.moveValueDown', { value1: skill.name ?? skill.skillId })}
            title={t('ui.moveDown')} disabled={index === skills.length - 1}
            onClick={() => {
              const pairs = asPairs();
              [pairs[index], pairs[index + 1]] = [pairs[index + 1]!, pairs[index]!];
              dispatch(pairs);
            }}>↓</button>
          <button type="button" className="skill-remove" aria-label={t('ui.removeValue', { value1: skill.name ?? skill.skillId })}
            title={t('ui.removeSkill')}
            onClick={() => dispatch(asPairs().filter((pair) => pair.id !== skill.skillId))}>×</button>
        </div>
      </li>)}
    </ul>
    <div className="skill-add-row">
      <SkillCombobox choices={available} mastery={pendingMastery} onChange={(id) =>
        dispatch([...asPairs(), { id, mastery: pendingMastery }])} />
      <label className="skill-mastery-label">{t('ui.as')}<select className="skill-mastery" aria-label={t('ui.masteryForTheNewSkill')}
          value={pendingMastery}
          onChange={(event) => setPendingMastery(Number(event.target.value))}>
          {MASTERIES.map((mastery) => <option key={mastery.value} value={mastery.value}>
            {mastery.label}</option>)}
        </select>
      </label>
    </div>
  </div>;
}
