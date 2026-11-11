import type { HeroSkillDto } from '../types/api';

/** One absolute SetSkills list: preserve mastery and position, validate before dispatch. */
export function replaceSkill(skills: HeroSkillDto[], index: number, id: number, supportedIds: number[]) {
  if (!Number.isInteger(index) || index < 0 || index >= skills.length) throw new Error('Invalid skill slot');
  if (!supportedIds.includes(id)) throw new Error('Unsupported skill');
  if (skills.some((skill, position) => position !== index && skill.skillId === id)) throw new Error('Duplicate skill');
  return skills.map((skill, position) => ({ id: position === index ? id : skill.skillId, mastery: skill.mastery }));
}
