// Copyright 2020 LEXUGE
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use super::{
    actions::{builder::ActionBuilder, Action, Result as ActionResult},
    matchers::builder::MatcherBuilder,
    Result, Rule,
};
use crate::Label;
use serde::{
    de::{Deserializer, Error as _, SeqAccess, Visitor},
    Deserialize, Serialize,
};
use std::marker::PhantomData;

/// A parsed branch of a rule.
#[derive(Serialize, Clone)]
pub struct BranchBuilder<A: ActionBuilder> {
    seq: Vec<A>,
    next: Label,
}

// This customized deserialization process accept branches of this form:
// ```
// - Action1
// - Action2
// - ...
// - next
// ```
// Here the lifetime constraints are compatible with the ones from serde derivation. We are not adding them to `AggregatedActionBuilder` as they are gonna be automatically generated by serde.
impl<'de, A: ActionBuilder + Deserialize<'de>> Deserialize<'de> for BranchBuilder<A> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Either<A: ActionBuilder> {
            Action(A),
            Tag(Label),
        }

        struct BranchVisitor<A> {
            // Dummy variable for visitor to be constrained by `A`.
            t: PhantomData<A>,
        }

        impl<'de, A: ActionBuilder + Deserialize<'de>> Visitor<'de> for BranchVisitor<A> {
            type Value = BranchBuilder<A>;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a list of actions with the tag of the next rule as the last element")
            }

            fn visit_seq<V: SeqAccess<'de>>(
                self,
                mut sv: V,
            ) -> std::result::Result<Self::Value, V::Error> {
                let mut seq = Vec::new();

                // Get the `next` from the first element of the type Label.
                let next = loop {
                    match sv.next_element::<Either<A>>() {
                        Ok(Some(Either::Action(a))) => seq.push(a),
                        Ok(Some(Either::Tag(l))) => break l,
                        Ok(None) => {
                            return Err(V::Error::custom("Missing the tag of the next rule"))
                        }
                        Err(_) => {
                            return Err(V::Error::custom(
                                "Make sure elements in branch are either valid actions or label in the end. Failed to parse the branch",
                            ))
                        }
                    }
                };

                // Verify that this is indeed the last element.
                if sv.next_element::<Either<A>>()?.is_some() {
                    return Err(V::Error::custom(
                        "Extra element after the rule tag specified at last in the rule definition",
                    ));
                }

                Ok(Self::Value { seq, next })
            }
        }

        deserializer.deserialize_seq(BranchVisitor::<A> { t: PhantomData })
    }
}

impl<A: ActionBuilder> BranchBuilder<A> {
    /// Create a new BranchBuilder from a sequence of actions and the destination tag name.
    pub fn new(seq: Vec<A>, next: impl Into<Label>) -> Self {
        Self {
            seq,
            next: next.into(),
        }
    }

    /// Build the ParMatchArm into the internal-used tuple by `Rule`.
    pub async fn build(self) -> ActionResult<(Vec<Box<dyn Action>>, Label)> {
        let mut built: Vec<Box<dyn Action>> = Vec::new();
        for a in self.seq {
            // TODO: Can we make this into a map?
            built.push(a.build().await?);
        }
        Ok((built, self.next))
    }
}

impl<A: ActionBuilder> Default for BranchBuilder<A> {
    fn default() -> Self {
        Self {
            seq: vec![],
            next: "end".into(),
        }
    }
}

/// A rule composed of tag name, matcher, and branches.
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
#[serde(deny_unknown_fields)]
pub struct RuleBuilder<M: MatcherBuilder, A: ActionBuilder> {
    /// The matcher rule uses.
    #[serde(rename = "if")]
    pub matcher: M,

    /// If matcher matches, this branch specifies action and next rule name to route. Defaut to `(Vec::new(), "end".into())`
    #[serde(default = "BranchBuilder::default")]
    #[serde(rename = "then")]
    pub on_match: BranchBuilder<A>,

    /// If matcher doesn't, this branch specifies action and next rule name to route. Defaut to `(Vec::new(), "end".into())`
    #[serde(default = "BranchBuilder::default")]
    #[serde(rename = "else")]
    pub no_match: BranchBuilder<A>,
}

impl<M: MatcherBuilder, A: ActionBuilder> RuleBuilder<M, A> {
    /// Create a new RuleBuilder
    pub fn new(matcher: M, on_match: BranchBuilder<A>, no_match: BranchBuilder<A>) -> Self {
        Self {
            matcher,
            on_match,
            no_match,
        }
    }

    /// Build a Rule out of a given RuleBuilder
    pub async fn build(self) -> Result<Rule> {
        let matcher = self.matcher.build().await?;
        let on_match = self.on_match.build().await?;
        let no_match = self.no_match.build().await?;
        Ok(Rule::new(matcher, on_match, no_match))
    }
}
