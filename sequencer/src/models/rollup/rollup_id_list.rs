use crate::models::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct RollupIdListModel {
    rollup_id_list: RollupIdList,
}

impl RollupIdListModel {
    pub fn new(rollup_id_list: RollupIdList) -> Self {
        Self { rollup_id_list }
    }

    pub fn is_exist_rollup_id(&self, rollup_id: &RollupId) -> bool {
        self.rollup_id_list.contains(rollup_id)
    }

    pub fn rollup_id_list(&self) -> &RollupIdList {
        &self.rollup_id_list
    }

    pub fn add_rollup_id(&mut self, rollup_id: RollupId) {
        let is_exist_rollup_id = self.rollup_id_list.contains(&rollup_id);

        if !is_exist_rollup_id {
            self.rollup_id_list.push(rollup_id);
        }
    }
}

impl RollupIdListModel {
    pub const ID: &'static str = stringify!(RollupIdListModel);

    pub fn get() -> Result<Self, DbError> {
        let key = Self::ID;
        match database()?.get(&key) {
            Ok(rollup_id_list_model) => Ok(rollup_id_list_model),
            Err(error) => {
                if error.is_none_type() {
                    let rollup_id_list_model = Self::new(RollupIdList::default());

                    rollup_id_list_model.put()?;

                    Ok(rollup_id_list_model)
                } else {
                    Err(error)
                }
            }
        }
    }

    pub fn entry() -> Result<Lock<'static, Self>, DbError> {
        let key = Self::ID;
        match database()?.get_mut(&key) {
            Ok(lock) => Ok(lock),
            Err(error) => {
                if error.is_none_type() {
                    let rollup_id_list_model = Self::new(RollupIdList::default());

                    rollup_id_list_model.put()?;

                    Ok(database()?.get_mut(&key)?)
                } else {
                    Err(error)
                }
            }
        }
    }

    pub fn put(&self) -> Result<(), DbError> {
        let key = Self::ID;
        database()?.put(&key, self)
    }
}