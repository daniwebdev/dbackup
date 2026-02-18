# DBackup Documentation (Fumadocs)

> Clean, beginner-friendly, and comprehensive documentation for DBackup - built with Fumadocs

## 📚 Quick Start

### View Documentation Locally

```bash
# Install dependencies
npm install

# Start development server
npm run dev
```

Open [http://localhost:3000](http://localhost:3000) to view the documentation.

### Build for Production

```bash
# Build static site
npm run build

# Serve production build
npm start
```

## 📁 Structure

```
/docs
├── app/              # Next.js app router
├── components/       # React components
├── content/
│   └── docs/        # All documentation content
│       ├── index.mdx                    (Home)
│       ├── getting-started.mdx          (Quick start)
│       ├── installation.mdx             (Installation guide)
│       ├── configuration.mdx            (Configuration)
│       ├── backup-modes.mdx             (Basic vs Parallel)
│       ├── storage.mdx                  (Local & S3)
│       ├── scheduling.mdx               (Cron scheduling)
│       ├── retention.mdx                (Auto cleanup)
│       ├── restore.mdx                  (Restoration guide)
│       ├── systemd-service.mdx          (Service setup)
│       ├── cli-reference.mdx            (CLI commands)
│       ├── troubleshooting.mdx          (Issue fixes)
│       └── meta.json                    (Navigation)
├── lib/              # Utility functions
├── next.config.mjs   # Next.js config
└── source.config.ts  # Fumadocs config
```

## ✍️ Writing Documentation

### Add New Page

1. **Create MDX file** in `/content/docs/`

```mdx
---
title: Your Page Title
description: Short description for search/preview
---

# Your Page Title

Content goes here...
```

2. **Update navigation** in `/content/docs/meta.json`

```json
{
  "title": "Your Page",
  "url": "/docs/your-page"
}
```

### Markdown/MDX Syntax

**Headings:**
```mdx
# H1 - Page Title
## H2 - Main section
### H3 - Subsection
```

**Cards (for navigation):**
```mdx
<Cards>
  <Card 
    title="Card Title" 
    description="Short description"
    href="/docs/page"
  />
</Cards>
```

**Callouts (warning, info, tip):**
```mdx
<Callout title="Title" type="tip">
  Content here
</Callout>
```

**Code blocks:**
````mdx
```bash
# Bash code
dbackup backup -c backup.yml
```

```yaml
# YAML code
settings:
  binary:
    pg_dump: /usr/bin/pg_dump
```
````

**Tables:**
```mdx
| Feature | Basic | Parallel |
|---------|-------|----------|
| Speed | Moderate | Fast |
| CPU | Low | High |
```

## 🎨 Components

Available Fumadocs components:

- `<Card>` - Navigation cards
- `<Cards>` - Card container
- `<Callout>` - Info/warning boxes
- `<Tab>` - Tabbed content
- Code blocks with syntax highlighting

## 🔍 Search

Search functionality is built-in and indexes:
- Page titles
- Headings
- Content text

## 🌙 Dark/Light Mode

Automatically supported by Fumadocs theme.

## 📱 Responsive Design

Documentation automatically adapts to:
- Desktop
- Tablet
- Mobile

## 🚀 Deployment

### Vercel (Recommended)

```bash
# Link to Vercel
vercel

# Deploy
vercel deploy --prod
```

### Static Hosting

```bash
# Build static files
npm run build

# Files in `.next/static` can be served as static content
```

### Docker

```dockerfile
FROM node:18-alpine

WORKDIR /app
COPY . .

RUN npm install
RUN npm run build

EXPOSE 3000
CMD ["npm", "start"]
```

## 🛠️ Development

### Add Custom Components

Edit `/components/mdx-components.tsx`:

```tsx
export const components = {
  // Your custom components
};
```

### Customize Theme

Theme configuration in `/lib/source.ts`:

```ts
export const source = loader({
  baseUrl: "/docs",
  // Theme customization options
});
```

## 📝 Content Guidelines

### Style Guide

- **Use active voice**: "Configure your backup" not "Backups can be configured"
- **Be concise**: Short paragraphs, clear sentences
- **Add examples**: Real-world code examples for every concept
- **Use tables**: For comparisons and quick reference
- **Link related**: Cross-reference related pages

### Structure each page:

1. **Introduction**: What is this about?
2. **Prerequisites**: What do I need?
3. **Step-by-step**: How do I do it?
4. **Examples**: Real-world use cases
5. **Troubleshooting**: Common issues
6. **Best practices**: Pro tips
7. **Next steps**: Where to go next

## 🔗 Useful Links

- [Fumadocs Documentation](https://fumadocs.dev)
- [MDX Syntax](https://mdxjs.com)
- [Next.js Documentation](https://nextjs.org/docs)
- [React Documentation](https://react.dev)

## 📋 Checklist before publishing

- ✅ All links work (internal and external)
- ✅ Code examples are tested
- ✅ No typos or grammatical errors
- ✅ Images optimized and properly sized
- ✅ Mobile responsive (test on phone)
- ✅ Dark mode tested
- ✅ Search working
- ✅ Navigation logical
- ✅ All pages in sidebar
- ✅ Metadata (titles, descriptions) present

## 🤝 Contributing

When adding documentation:

1. Write in Markdown/MDX
2. Follow style guide
3. Add examples
4. Update navigation
5. Test locally
6. Submit for review

## 📄 License

Documentation is part of DBackup project license.

---

**Last updated**: February 18, 2026
**Framework**: Fumadocs + Next.js
**Status**: Complete & Ready for Production
