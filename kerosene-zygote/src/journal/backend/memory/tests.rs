use futures::{executor::block_on, join};

use super::*;

fn block(data: &'static [u8]) -> Bytes {
    Bytes::from_static(data)
}

#[test]
fn create_append_and_read() {
    block_on(async {
        let backend = MemoryBackend::new();

        let (id, metadata) = backend.create(Config::default()).await.unwrap();
        assert_eq!(metadata.end, 0);
        assert_eq!(metadata.size, 0);

        assert_eq!(backend.append(id, block(b"hello")).await.unwrap(), 0);
        assert_eq!(backend.append(id, block(b"world!")).await.unwrap(), 1);

        let metadata = backend.stat(id).await.unwrap();
        assert_eq!(metadata.end, 2);
        assert_eq!(metadata.size, 11);
        assert_eq!(metadata.physical_size, 11);

        let blocks: Vec<_> = backend.read(id, 0, false).await.unwrap().collect().await;
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].as_ref().unwrap().seq, 0);
        assert_eq!(blocks[1].as_ref().unwrap().data, b"world!"[..]);

        let journals: Vec<_> = backend.list().collect().await;
        assert_eq!(journals.len(), 1);
        assert_eq!(journals[0].as_ref().unwrap().0, id);
    });
}

#[test]
fn finite_reads_use_a_snapshot() {
    block_on(async {
        let backend = MemoryBackend::new();
        let (id, _) = backend.create(Config::default()).await.unwrap();
        backend.append(id, block(b"a")).await.unwrap();

        let stream = backend.read(id, 0, false).await.unwrap();
        backend.append(id, block(b"b")).await.unwrap();

        let blocks: Vec<_> = stream.collect().await;
        assert_eq!(blocks.len(), 1);
    });
}

#[test]
fn freeze_then_delete() {
    block_on(async {
        let backend = MemoryBackend::new();
        let (id, _) = backend.create(Config::default()).await.unwrap();

        assert!(backend.freeze(id).await.unwrap());
        assert!(!backend.freeze(id).await.unwrap());
        assert!(backend.stat(id).await.unwrap().frozen);
        assert!(matches!(
            backend.append(id, block(b"nope")).await,
            Err(WriteError::Frozen)
        ));

        backend.delete(id).await.unwrap();
        assert!(matches!(backend.stat(id).await, Err(ReadError::Deleted)));
        assert!(matches!(
            backend.append(id, block(b"nope")).await,
            Err(WriteError::Deleted)
        ));
        assert!(matches!(backend.open(id).await, Err(OpenError::Missing)));
        assert_eq!(backend.list().collect::<Vec<_>>().await.len(), 0);
    });
}

#[test]
fn meta_journal_is_read_only() {
    block_on(async {
        let backend = MemoryBackend::new();

        assert!(matches!(
            backend.append(JournalId::META, block(b"nope")).await,
            Err(WriteError::MetaWrite)
        ));
        assert!(matches!(
            backend.freeze(JournalId::META).await,
            Err(WriteError::MetaFreeze)
        ));
    });
}

#[test]
fn deleted_meta_is_recreated() {
    block_on(async {
        let backend = MemoryBackend::new();
        backend.delete(JournalId::META).await.unwrap();
        assert!(matches!(
            backend.stat(JournalId::META).await,
            Err(ReadError::Deleted)
        ));

        backend.create(Config::default()).await.unwrap();
        let meta = backend.stat(JournalId::META).await.unwrap();
        assert_eq!(meta.end, 1);
    });
}

#[test]
fn tail_waits_for_appends_and_deletes() {
    block_on(async {
        let backend = MemoryBackend::new();
        let (id, _) = backend.create(Config::default()).await.unwrap();
        let mut tail = backend.read(id, 0, true).await.unwrap();

        let (block, _) = join!(tail.next(), async {
            backend.append(id, block(b"hello")).await.unwrap();
        });
        assert_eq!(block.unwrap().unwrap().seq, 0);

        let (block, _) = join!(tail.next(), async {
            backend.delete(id).await.unwrap();
        });
        assert!(matches!(block, Some(Err(ReadError::Deleted))));
    });
}
