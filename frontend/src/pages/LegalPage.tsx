import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import communityRulesMarkdown from '../markdown/legal/community_rules.md?raw'
import ctfRulesMarkdown from '../markdown/legal/ctf_rules.md?raw'
import privacyPolicyMarkdown from '../markdown/legal/privacy_policy.md?raw'
import projectRulesMarkdown from '../markdown/legal/project_rules.md?raw'
import publicOfferMarkdown from '../markdown/legal/public_offer.md?raw'
import refundPolicyMarkdown from '../markdown/legal/refund_policy.md?raw'
import userAgreementMarkdown from '../markdown/legal/user_agreement.md?raw'
import { paths } from '../routes/paths'
import './LegalPage.css'

type LegalDocument = {
  eyebrow: string
  title: string
  markdown: string
}

type LegalDocumentEntry = LegalDocument & {
  path: string
}

type LegalDocumentGroup = {
  title: string
  documents: LegalDocumentEntry[]
}

const legalDocuments: Record<string, LegalDocumentEntry> = {
  [paths.legalPrivacyPolicy]: {
    path: paths.legalPrivacyPolicy,
    eyebrow: 'Legal',
    title: 'Privacy Policy',
    markdown: privacyPolicyMarkdown,
  },
  [paths.legalPublicOffer]: {
    path: paths.legalPublicOffer,
    eyebrow: 'Legal',
    title: 'Public Offer',
    markdown: publicOfferMarkdown,
  },
  [paths.legalRefundPolicy]: {
    path: paths.legalRefundPolicy,
    eyebrow: 'Legal',
    title: 'Refund Policy',
    markdown: refundPolicyMarkdown,
  },
  [paths.legalUserAgreement]: {
    path: paths.legalUserAgreement,
    eyebrow: 'Legal',
    title: 'User Agreement',
    markdown: userAgreementMarkdown,
  },
  [paths.legalProjectRules]: {
    path: paths.legalProjectRules,
    eyebrow: 'Правила',
    title: 'Общие правила проекта',
    markdown: projectRulesMarkdown,
  },
  [paths.legalCommunityRules]: {
    path: paths.legalCommunityRules,
    eyebrow: 'Правила',
    title: 'Правила сообщества',
    markdown: communityRulesMarkdown,
  },
  [paths.legalCtfRules]: {
    path: paths.legalCtfRules,
    eyebrow: 'Правила',
    title: 'Правила режима CTF',
    markdown: ctfRulesMarkdown,
  },
}

const legalDocumentGroups: LegalDocumentGroup[] = [
  {
    title: 'Юридические документы',
    documents: [
      legalDocuments[paths.legalPrivacyPolicy],
      legalDocuments[paths.legalPublicOffer],
      legalDocuments[paths.legalRefundPolicy],
      legalDocuments[paths.legalUserAgreement],
    ],
  },
  {
    title: 'Правила',
    documents: [
      legalDocuments[paths.legalProjectRules],
      legalDocuments[paths.legalCommunityRules],
      legalDocuments[paths.legalCtfRules],
    ],
  },
]

// eslint-disable-next-line react-refresh/only-export-components
export function isLegalPath(pathname: string) {
  return pathname === paths.legal || pathname in legalDocuments
}

// eslint-disable-next-line react-refresh/only-export-components
export function legalPageHeading(pathname: string) {
  if (pathname === paths.legal) {
    return 'Правовые документы'
  }

  const headings: Record<string, string> = {
    [paths.legalPrivacyPolicy]: 'Политика конфиденциальности',
    [paths.legalPublicOffer]: 'Публичная оферта',
    [paths.legalRefundPolicy]: 'Политика возвратов',
    [paths.legalUserAgreement]: 'Пользовательское соглашение',
    [paths.legalProjectRules]: 'Общие правила проекта',
    [paths.legalCommunityRules]: 'Правила сообщества',
    [paths.legalCtfRules]: 'Правила режима CTF',
  }

  return headings[pathname] ?? 'Правовые документы'
}

export default function LegalPage({ pathname }: { pathname: string }) {
  if (pathname === paths.legal) {
    return (
      <div className="page legal-page">
        <section className="card legal-page__card">
          <span className="legal-page__eyebrow">Legal</span>
          <h1 className="legal-page__title">Правовые документы</h1>
          <p className="legal-page__lead">
            Здесь собраны документы, которые регулируют использование сервиса и покупки внутри проекта.
          </p>
          <div className="legal-page__groups">
            {legalDocumentGroups.map((group) => (
              <section
                key={group.title}
                className="legal-page__group"
                aria-labelledby={`legal-group-${group.title}`}
              >
                <h2 id={`legal-group-${group.title}`} className="legal-page__section-title">
                  {group.title}
                </h2>
                <nav aria-label={group.title}>
                  <ul className="legal-page__list">
                    {group.documents.map((document) => (
                      <li key={document.path} className="legal-page__list-item">
                        <a href={document.path} className="legal-page__list-link">
                          <span className="legal-page__list-title">{document.title}</span>
                          <span className="legal-page__list-arrow" aria-hidden>→</span>
                        </a>
                      </li>
                    ))}
                  </ul>
                </nav>
              </section>
            ))}
          </div>
        </section>
      </div>
    )
  }

  const document = legalDocuments[pathname] ?? legalDocuments[paths.legalPrivacyPolicy]

  return (
    <div className="page legal-page">
      <article className="card legal-page__card">
        <span className="legal-page__eyebrow">{document.eyebrow}</span>
        <nav className="legal-page__subnav" aria-label="Другие документы">
          <a className="legal-page__subnav-home" href={paths.legal}>Все документы</a>
          {legalDocumentGroups.map((group) => (
            <div key={group.title} className="legal-page__subnav-row" aria-label={group.title}>
              <span className="legal-page__subnav-label">{group.title}</span>
              <div className="legal-page__subnav-links">
                {group.documents.map((entry) => (
                  <a
                    key={entry.path}
                    href={entry.path}
                    aria-current={entry.path === pathname ? 'page' : undefined}
                  >
                    {entry.title}
                  </a>
                ))}
              </div>
            </div>
          ))}
        </nav>
        <div className="legal-page__content">
          <ReactMarkdown remarkPlugins={[remarkGfm]}>{document.markdown}</ReactMarkdown>
        </div>
      </article>
    </div>
  )
}
