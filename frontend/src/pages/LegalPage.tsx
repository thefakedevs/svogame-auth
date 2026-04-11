import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import privacyPolicyMarkdown from '../markdown/legal/privacy_policy.md?raw'
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
}

const legalDocumentList = [
  legalDocuments[paths.legalPrivacyPolicy],
  legalDocuments[paths.legalPublicOffer],
  legalDocuments[paths.legalRefundPolicy],
  legalDocuments[paths.legalUserAgreement],
]

export function isLegalPath(pathname: string) {
  return pathname === paths.legal || pathname in legalDocuments
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
          <nav aria-label="Список правовых документов">
            <ul className="legal-page__list">
              {legalDocumentList.map((document) => (
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
      </div>
    )
  }

  const document = legalDocuments[pathname] ?? legalDocuments[paths.legalPrivacyPolicy]

  return (
    <div className="page legal-page">
      <article className="card legal-page__card">
        <span className="legal-page__eyebrow">{document.eyebrow}</span>
        <nav className="legal-page__subnav" aria-label="Другие правовые документы">
          <a href={paths.legal}>Все документы</a>
          {legalDocumentList
            .filter((entry) => entry.path !== pathname)
            .map((entry) => (
              <a key={entry.path} href={entry.path}>{entry.title}</a>
            ))}
        </nav>
        <div className="legal-page__content">
          <ReactMarkdown remarkPlugins={[remarkGfm]}>{document.markdown}</ReactMarkdown>
        </div>
      </article>
    </div>
  )
}
