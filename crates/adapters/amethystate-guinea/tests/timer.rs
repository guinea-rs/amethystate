use std::time::Duration;

use amethystate::Field;
use amethystate::store::field_with_path;
use amethystate::test_utils::unique_store;
use amethystate::uuid::Uuid;
use amethystate_guinea::IntoChanging;
use guinea::app::Harness;
use guinea::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Count(pub usize);

#[reducer]
fn count(this: &mut Count, _: ()) {
    this.0 += 1;
}

pub struct Ping;

pub struct Counter {
    push: Push<Count>,
}

actor! {
    Counter {
        handlers { Ping }
    }
}

#[handler]
fn ping(In(this, _, _): In<Counter, Ping>) {
    this.push.send(());
}

feature! {
    pub Polling {
        exports { Count }
    }
}

#[installs]
fn polling(cx: &Install<Field<u64>>) -> anyhow::Result<Polling> {
    let (count, agent) = cx.state::<Count>().driven_by(|push| Counter { push });
    cx.every(
        Period::follows(cx.params().clone().changing(), Duration::from_millis),
        || Ping,
        agent.clone(),
    );
    Ok(Polling(count))
}

#[guinea::test]
fn a_field_period_sends_messages_to_an_actor(h: &mut Harness) {
    let (_at, store) = unique_store("guinea-actor-period");
    let interval = field_with_path(&store, ["net", "ping_ms"], 500u64, Uuid::new_v4()).unwrap();
    h.install::<Polling>(&interval).unwrap();
    h.settled();

    h.advance(Duration::from_millis(500));
    assert_eq!(h.state::<Count>().0, 1);

    interval.set(250);
    h.settled();
    h.advance(Duration::from_millis(250));
    assert_eq!(h.state::<Count>().0, 2);
}
