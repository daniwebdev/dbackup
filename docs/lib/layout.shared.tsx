import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';

// fill this with your actual GitHub info, for example:
export const gitConfig = {
  user: 'daniwebdev',
  repo: 'dbackup',
  branch: 'main',
};

export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: 'DBackup',
    },
    githubUrl: `https://github.com/${gitConfig.user}/${gitConfig.repo}`,
  };
}
