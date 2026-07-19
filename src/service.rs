use std::sync::Arc;

use crate::model::{Account, NewTransaction, Transaction};
use crate::repository::{AccountRepo, RepoError, TransactionRepo};

/// Errors surfaced by the service layer, carrying enough context for the API
/// layer to pick a status code and build an error body.
#[derive(Debug, PartialEq)]
pub enum ServiceError {
    /// operation_type_id outside 1..=4
    InvalidOperation,
    /// account_id does not reference an existing account
    AccountNotFound,
    /// document_number already registered
    AccountAlreadyExists,
    Internal(String),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::InvalidOperation => {
                write!(f, "operation_type_id must be between 1 and 4")
            }
            ServiceError::AccountNotFound => write!(f, "account not found"),
            ServiceError::AccountAlreadyExists => write!(f, "account already exists"),
            ServiceError::Internal(details) => write!(f, "{details}"),
        }
    }
}

impl std::error::Error for ServiceError {}

// ----------------------
// Account service
// ----------------------

#[derive(Clone)]
pub struct AccountService {
    acc_repo: Arc<dyn AccountRepo>,
}

impl AccountService {
    pub fn new(acc_repo: Arc<dyn AccountRepo>) -> Self {
        Self { acc_repo }
    }

    pub async fn create_account(&self, document_number: &str) -> Result<Account, ServiceError> {
        if self
            .acc_repo
            .get_by_document_number(document_number)
            .await
            .is_ok()
        {
            return Err(ServiceError::AccountAlreadyExists);
        }
        self.acc_repo
            .create_account(document_number)
            .await
            .map_err(|e| ServiceError::Internal(format!("failed to create account: {e}")))
    }

    pub async fn get_account_by_id(&self, id: i64) -> Result<Account, ServiceError> {
        self.acc_repo.get_by_id(id).await.map_err(|e| match e {
            RepoError::NotFound => ServiceError::AccountNotFound,
            other => ServiceError::Internal(format!("failed to fetch account: {other}")),
        })
    }
}

// ----------------------
// Transaction service
// ----------------------

#[derive(Clone)]
pub struct TransactionService {
    acc_repo: Arc<dyn AccountRepo>,
    tx_repo: Arc<dyn TransactionRepo>,
}

impl TransactionService {
    pub fn new(acc_repo: Arc<dyn AccountRepo>, tx_repo: Arc<dyn TransactionRepo>) -> Self {
        Self { acc_repo, tx_repo }
    }

    pub async fn create_transaction(
        &self,
        mut t: NewTransaction,
    ) -> Result<Transaction, ServiceError> {
        if self.acc_repo.get_by_id(t.account_id).await.is_err() {
            return Err(ServiceError::AccountNotFound);
        }

        match t.operation_type_id {
            // debit operations are stored as negative amounts
            1..=3 => {
                if t.amount >= 0.0 {
                    t.amount = -t.amount;
                }
            }
            // payments are stored as positive amounts
            4 => {
                if t.amount <= 0.0 {
                    t.amount = -t.amount;
                }
            }
            _ => return Err(ServiceError::InvalidOperation),
        }

        self.tx_repo
            .create_transaction(t)
            .await
            .map_err(|e| ServiceError::Internal(format!("failed to create transaction: {e}")))
    }

    pub async fn get_balance(&self, account_id: i64) -> Result<f64, ServiceError> {
        self.acc_repo
            .get_by_id(account_id)
            .await
            .map_err(|e| match e {
                RepoError::NotFound => ServiceError::AccountNotFound,
                other => ServiceError::Internal(format!("failed to fetch account: {other}")),
            })?;

        self.tx_repo
            .get_balance_by_account(account_id)
            .await
            .map_err(|e| ServiceError::Internal(format!("failed to fetch balance: {e}")))
    }
}

// ----------------------
// Unit tests with mock repositories
// ----------------------

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use std::sync::Mutex;

    struct MockRepo {
        accounts: Mutex<Vec<Account>>,
        transactions: Mutex<Vec<Transaction>>,
    }

    impl MockRepo {
        fn new() -> Self {
            Self {
                accounts: Mutex::new(vec![]),
                transactions: Mutex::new(vec![]),
            }
        }

        fn with_account(id: i64, doc: &str) -> Self {
            let repo = Self::new();
            repo.accounts.lock().unwrap().push(Account {
                account_id: id,
                document_number: doc.to_string(),
            });
            repo
        }
    }

    #[async_trait]
    impl AccountRepo for MockRepo {
        async fn create_account(&self, document_number: &str) -> Result<Account, RepoError> {
            let mut accounts = self.accounts.lock().unwrap();
            let account = Account {
                account_id: accounts.len() as i64 + 1,
                document_number: document_number.to_string(),
            };
            accounts.push(account.clone());
            Ok(account)
        }

        async fn get_by_id(&self, id: i64) -> Result<Account, RepoError> {
            self.accounts
                .lock()
                .unwrap()
                .iter()
                .find(|a| a.account_id == id)
                .cloned()
                .ok_or(RepoError::NotFound)
        }

        async fn get_by_document_number(&self, doc: &str) -> Result<Account, RepoError> {
            self.accounts
                .lock()
                .unwrap()
                .iter()
                .find(|a| a.document_number == doc)
                .cloned()
                .ok_or(RepoError::NotFound)
        }
    }

    #[async_trait]
    impl TransactionRepo for MockRepo {
        async fn create_transaction(&self, t: NewTransaction) -> Result<Transaction, RepoError> {
            let mut transactions = self.transactions.lock().unwrap();
            let tx = Transaction {
                transaction_id: transactions.len() as i64 + 1,
                account_id: t.account_id,
                operation_type_id: t.operation_type_id,
                amount: t.amount,
                destination_account_id: t.destination_account_id,
                event_date: Utc::now(),
            };
            transactions.push(tx.clone());
            Ok(tx)
        }

        async fn get_balance_by_account(&self, account_id: i64) -> Result<f64, RepoError> {
            let transactions = self.transactions.lock().unwrap();
            let source: f64 = transactions
                .iter()
                .filter(|t| t.account_id == account_id)
                .map(|t| t.amount)
                .sum();
            let destination: f64 = transactions
                .iter()
                .filter(|t| t.destination_account_id == account_id)
                .map(|t| t.amount)
                .sum();
            Ok(source - destination)
        }
    }

    fn tx_service(repo: Arc<MockRepo>) -> TransactionService {
        TransactionService::new(repo.clone(), repo)
    }

    #[tokio::test]
    async fn create_account_succeeds() {
        let svc = AccountService::new(Arc::new(MockRepo::new()));
        let account = svc.create_account("12345678900").await.unwrap();
        assert_eq!(account.account_id, 1);
        assert_eq!(account.document_number, "12345678900");
    }

    #[tokio::test]
    async fn create_account_rejects_duplicate_document_number() {
        let svc = AccountService::new(Arc::new(MockRepo::with_account(1, "12345678900")));
        let err = svc.create_account("12345678900").await.unwrap_err();
        assert_eq!(err, ServiceError::AccountAlreadyExists);
    }

    #[tokio::test]
    async fn get_account_by_id_returns_not_found_for_missing_account() {
        let svc = AccountService::new(Arc::new(MockRepo::new()));
        let err = svc.get_account_by_id(42).await.unwrap_err();
        assert_eq!(err, ServiceError::AccountNotFound);
    }

    #[tokio::test]
    async fn purchase_amount_is_stored_as_negative() {
        let svc = tx_service(Arc::new(MockRepo::with_account(1, "doc")));
        for op in 1..=3 {
            let tx = svc
                .create_transaction(NewTransaction {
                    account_id: 1,
                    operation_type_id: op,
                    amount: 100.0,
                    destination_account_id: 2,
                })
                .await
                .unwrap();
            assert_eq!(tx.amount, -100.0, "operation {op} should debit");
        }
    }

    #[tokio::test]
    async fn payment_amount_is_stored_as_positive() {
        let svc = tx_service(Arc::new(MockRepo::with_account(1, "doc")));
        let tx = svc
            .create_transaction(NewTransaction {
                account_id: 1,
                operation_type_id: 4,
                amount: -123.45,
                destination_account_id: 2,
            })
            .await
            .unwrap();
        assert_eq!(tx.amount, 123.45);
    }

    #[tokio::test]
    async fn invalid_operation_type_is_rejected() {
        let svc = tx_service(Arc::new(MockRepo::with_account(1, "doc")));
        for op in [0, 5, -1] {
            let err = svc
                .create_transaction(NewTransaction {
                    account_id: 1,
                    operation_type_id: op,
                    amount: 10.0,
                    destination_account_id: 2,
                })
                .await
                .unwrap_err();
            assert_eq!(err, ServiceError::InvalidOperation);
        }
    }

    #[tokio::test]
    async fn transaction_for_unknown_account_is_rejected() {
        let svc = tx_service(Arc::new(MockRepo::new()));
        let err = svc
            .create_transaction(NewTransaction {
                account_id: 99,
                operation_type_id: 1,
                amount: 10.0,
                destination_account_id: 2,
            })
            .await
            .unwrap_err();
        assert_eq!(err, ServiceError::AccountNotFound);
    }

    #[tokio::test]
    async fn balance_mirrors_destination_transactions() {
        let repo = Arc::new(MockRepo::with_account(1, "doc"));
        repo.accounts.lock().unwrap().push(Account {
            account_id: 2,
            document_number: "doc2".to_string(),
        });
        let svc = tx_service(repo.clone());

        // account 1 pays 100 to account 2: a debit for 1, a mirrored credit for 2.
        svc.create_transaction(NewTransaction {
            account_id: 1,
            operation_type_id: 1,
            amount: 100.0,
            destination_account_id: 2,
        })
        .await
        .unwrap();

        assert_eq!(svc.get_balance(1).await.unwrap(), -100.0);
        assert_eq!(svc.get_balance(2).await.unwrap(), 100.0);
    }
}
