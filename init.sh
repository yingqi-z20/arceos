cd /tmp
if_test_exist test_user rm test_user
if_test_exist test_user ./test_user
echo echo please run init with root > test_user
echo exit 1 >> test_user
chmod 555 test_user
chown root:root test_user
rm test_user
if_test_exist test_user ./test_user
echo TODO
exit 0
